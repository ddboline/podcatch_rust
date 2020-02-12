use anyhow::{format_err, Error};
use log::debug;
use postgres_query::FromSqlRow;
use reqwest::Url;
use std::fs::remove_file;
use std::path::Path;

use crate::episode_status::EpisodeStatus;
use crate::get_md5sum;
use crate::pgpool::PgPool;
use crate::pod_connection::PodConnection;

#[derive(Default, Clone, Debug, FromSqlRow)]
pub struct Episode {
    pub castid: i32,
    pub episodeid: i32,
    pub title: String,
    pub epurl: String,
    pub enctype: String,
    pub status: EpisodeStatus,
    pub epguid: Option<String>,
}

impl Episode {
    pub fn url_basename(&self) -> Result<String, Error> {
        if self.epurl.ends_with("media.mp3") {
            let basename: String = self
                .title
                .to_lowercase()
                .chars()
                .filter_map(|c| match c {
                    'a'..='z' | '0'..='9' => Some(c),
                    ' ' => Some('_'),
                    _ => None,
                })
                .collect();
            Ok(format!("{}.mp3", basename))
        } else if self.epurl.contains("newrustacean/") {
            let basename: Vec<_> = self
                .epurl
                .split("newrustacean/")
                .last()
                .ok_or_else(|| format_err!("..."))?
                .split('/')
                .collect();
            let basename: String = basename.join("_");
            Ok(basename)
        } else {
            let epurl: Url = self.epurl.parse()?;
            epurl
                .path()
                .split('/')
                .last()
                .map(ToString::to_string)
                .ok_or_else(|| format_err!("No basename"))
        }
    }

    pub fn from_index(pool: &PgPool, cid: i32, eid: i32) -> Result<Option<Self>, Error> {
        let query = r#"
            SELECT
                castid, episodeid, title, epurl, enctype, status, epguid
            FROM episodes
            WHERE castid = $1 AND episodeid = $2
        "#;
        if let Some(row) = pool.get()?.query(query, &[&cid, &eid])?.get(0) {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn from_epurl(pool: &PgPool, cid: i32, epurl: &str) -> Result<Option<Self>, Error> {
        let query = r#"
            SELECT
                castid, episodeid, title, epurl, enctype, status, epguid
            FROM episodes
            WHERE castid = $1 AND epurl = $2
        "#;
        if let Some(row) = pool.get()?.query(query, &[&cid, &epurl])?.get(0) {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn from_epguid(pool: &PgPool, cid: i32, epguid: &str) -> Result<Option<Self>, Error> {
        let query = r#"
            SELECT
                castid, episodeid, title, epurl, enctype, status, epguid
            FROM episodes
            WHERE castid = $1 AND epguid = $2
        "#;
        if let Some(row) = pool.get()?.query(query, &[&cid, &epguid])?.get(0) {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_episodes(pool: &PgPool, cid: i32) -> Result<Vec<Self>, Error> {
        let query = r#"
            SELECT
                castid, episodeid, title, epurl, enctype, status, epguid
            FROM episodes
            WHERE castid = $1
        "#;
        pool.get()?
            .query(query, &[&cid])?
            .iter()
            .map(|row| Ok(Self::from_row(row)?))
            .collect()
    }

    pub fn insert_episode(&self, pool: &PgPool) -> Result<u64, Error> {
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
        pool.get()?
            .execute(query.sql(), &query.parameters())
            .map_err(Into::into)
    }

    pub fn update_episode(&self, pool: &PgPool) -> Result<u64, Error> {
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
        pool.get()?
            .execute(query.sql(), &query.parameters())
            .map_err(Into::into)
    }

    pub fn get_max_epid(pool: &PgPool) -> Result<i32, Error> {
        let query = "SELECT MAX(episodeid) FROM episodes";
        pool.get()?
            .query(query, &[])?
            .get(0)
            .ok_or_else(|| format_err!("No episodes"))
            .and_then(|row| row.try_get(0).map_err(Into::into))
    }

    pub fn download_episode(&self, conn: &PodConnection, directory: &str) -> Result<Self, Error> {
        if !Path::new(directory).exists() {
            Err(format_err!("No such directory {}", directory))
        } else if let Ok(url) = self.epurl.parse() {
            let outfile = format!("{}/{}", directory, self.url_basename()?);
            if Path::new(&outfile).exists() {
                remove_file(&outfile)?;
            }
            conn.dump_to_file(&url, &outfile)?;
            let path = Path::new(&outfile);
            if path.exists() {
                let md5sum = get_md5sum(&path)?;
                let mut p = self.clone();
                debug!("{} {}", outfile, md5sum);
                p.epguid.replace(md5sum);
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
    use crate::config::Config;
    use crate::episode::Episode;
    use crate::pgpool::PgPool;

    #[test]
    #[ignore]
    fn test_episodes_get_all_episodes() {
        let config = Config::init_config().unwrap();
        let pool = PgPool::new(&config.database_url);

        let eps = Episode::get_all_episodes(&pool, 1).unwrap();

        assert!(eps.len() > 100);
    }
}
