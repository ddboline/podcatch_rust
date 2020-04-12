use anyhow::{format_err, Error};
use log::debug;
use postgres_query::FromSqlRow;
use reqwest::Url;
use std::path::Path;
use tokio::fs::remove_file;

use crate::{
    episode_status::EpisodeStatus, get_md5sum, pgpool::PgPool, pod_connection::PodConnection,
    stack_string::StackString,
};

#[derive(Default, Clone, Debug, FromSqlRow)]
pub struct Episode {
    pub castid: i32,
    pub episodeid: i32,
    pub title: StackString,
    pub epurl: StackString,
    pub enctype: StackString,
    pub status: EpisodeStatus,
    pub epguid: Option<StackString>,
}

impl Episode {
    pub fn url_basename(&self) -> Result<String, Error> {
        if self.epurl.as_str().ends_with("media.mp3")
            || self.epurl.as_str().contains("https://feeds.acast.com")
        {
            let basename: String = self
                .title
                .as_str()
                .to_lowercase()
                .chars()
                .filter_map(|c| match c {
                    'a'..='z' | '0'..='9' => Some(c),
                    ' ' => Some('_'),
                    _ => None,
                })
                .collect();
            Ok(format!("{}.mp3", basename))
        } else if self.epurl.as_str().contains("newrustacean/") {
            let basename: Vec<_> = self
                .epurl
                .as_str()
                .split("newrustacean/")
                .last()
                .ok_or_else(|| format_err!("..."))?
                .split('/')
                .collect();
            let basename: String = basename.join("_");
            Ok(basename)
        } else {
            let epurl: Url = self.epurl.as_str().parse()?;
            epurl
                .path()
                .split('/')
                .last()
                .map(ToString::to_string)
                .ok_or_else(|| format_err!("No basename"))
        }
    }

    pub async fn from_index(pool: &PgPool, cid: i32, eid: i32) -> Result<Option<Self>, Error> {
        let query = r#"
            SELECT
                castid, episodeid, title, epurl, enctype, status, epguid
            FROM episodes
            WHERE castid = $1 AND episodeid = $2
        "#;
        if let Some(row) = pool.get().await?.query(query, &[&cid, &eid]).await?.get(0) {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn from_epurl(pool: &PgPool, cid: i32, epurl: &str) -> Result<Option<Self>, Error> {
        let query = r#"
            SELECT
                castid, episodeid, title, epurl, enctype, status, epguid
            FROM episodes
            WHERE castid = $1 AND epurl = $2
        "#;
        if let Some(row) = pool
            .get()
            .await?
            .query(query, &[&cid, &epurl])
            .await?
            .get(0)
        {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn from_epguid(pool: &PgPool, cid: i32, epguid: &str) -> Result<Option<Self>, Error> {
        let query = r#"
            SELECT
                castid, episodeid, title, epurl, enctype, status, epguid
            FROM episodes
            WHERE castid = $1 AND epguid = $2
        "#;
        if let Some(row) = pool
            .get()
            .await?
            .query(query, &[&cid, &epguid])
            .await?
            .get(0)
        {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn get_all_episodes(pool: &PgPool, cid: i32) -> Result<Vec<Self>, Error> {
        let query = r#"
            SELECT
                castid, episodeid, title, epurl, enctype, status, epguid
            FROM episodes
            WHERE castid = $1
        "#;
        pool.get()
            .await?
            .query(query, &[&cid])
            .await?
            .iter()
            .map(|row| Ok(Self::from_row(row)?))
            .collect()
    }

    pub async fn insert_episode(&self, pool: &PgPool) -> Result<u64, Error> {
        let status = self.status.to_string();
        let query = postgres_query::query!(
            r#"
            INSERT INTO episodes (
                castid, episodeid, title, epurl, enctype, status, epguid
            ) VALUES (
                $castid, $episodeid, $title, $epurl, $enctype, $status, $epguid
            )
        "#,
            castid = self.castid,
            episodeid = self.episodeid,
            title = self.title,
            epurl = self.epurl,
            enctype = self.enctype,
            status = status,
            epguid = self.epguid
        );
        pool.get()
            .await?
            .execute(query.sql(), &query.parameters())
            .await
            .map_err(Into::into)
    }

    pub async fn update_episode(&self, pool: &PgPool) -> Result<u64, Error> {
        let status = self.status.to_string();
        let query = postgres_query::query!(
            r#"
                UPDATE episodes
                SET title=$title,epurl=$epurl,enctype=$enctype,status=$status,epguid=$epguid
                WHERE castid=$castid AND episodeid=$episodeid
            "#,
            castid = self.castid,
            episodeid = self.episodeid,
            title = self.title,
            epurl = self.epurl,
            enctype = self.enctype,
            status = status,
            epguid = self.epguid
        );
        pool.get()
            .await?
            .execute(query.sql(), &query.parameters())
            .await
            .map_err(Into::into)
    }

    pub async fn get_max_epid(pool: &PgPool) -> Result<i32, Error> {
        let query = "SELECT MAX(episodeid) FROM episodes";
        pool.get()
            .await?
            .query(query, &[])
            .await?
            .get(0)
            .ok_or_else(|| format_err!("No episodes"))
            .and_then(|row| row.try_get(0).map_err(Into::into))
    }

    pub async fn download_episode(
        &self,
        conn: &PodConnection,
        directory: &Path,
    ) -> Result<Self, Error> {
        if !directory.exists() {
            Err(format_err!(
                "No such directory {}",
                directory.to_string_lossy()
            ))
        } else if let Ok(url) = self.epurl.as_str().parse() {
            let outfile = directory.join(self.url_basename()?);
            if outfile.exists() {
                remove_file(&outfile).await?;
            }
            conn.dump_to_file(&url, &outfile).await?;
            let path = Path::new(&outfile);
            if path.exists() {
                let md5sum = get_md5sum(&path)?;
                let mut p = self.clone();
                debug!("{:?} {}", outfile, md5sum);
                p.epguid.replace(md5sum.into());
                p.status = EpisodeStatus::Downloaded;
                Ok(p)
            } else {
                Err(format_err!("Download failed"))
            }
        } else {
            Err(format_err!("Unkown failure {:?}", self))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{config::Config, episode::Episode, pgpool::PgPool};

    #[tokio::test]
    #[ignore]
    async fn test_episodes_get_all_episodes() {
        let config = Config::init_config().unwrap();
        let pool = PgPool::new(config.database_url.as_str());

        let eps = Episode::get_all_episodes(&pool, 1).await.unwrap();

        assert!(eps.len() > 100);
    }
}
