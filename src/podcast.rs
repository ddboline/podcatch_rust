use anyhow::Error;
use postgres_query::FromSqlRow;
use reqwest::Url;
use std::collections::HashMap;

use crate::{pgpool::PgPool, pod_connection::PodConnection, stack_string::StackString};

#[derive(Default, Clone, Debug, FromSqlRow)]
pub struct Podcast {
    pub castid: i32,
    pub castname: StackString,
    pub feedurl: StackString,
    pub directory: Option<StackString>,
}

impl Podcast {
    pub async fn add_podcast(
        pool: &PgPool,
        cid: i32,
        cname: &str,
        furl: &Url,
        dir: &str,
    ) -> Result<Self, Error> {
        let pod = if let Some(p) = Self::from_index(&pool, cid).await? {
            p
        } else if let Some(p) = Self::from_feedurl(&pool, furl.as_str()).await? {
            p
        } else {
            let pod = Self {
                castid: cid,
                castname: cname.into(),
                feedurl: furl.as_str().into(),
                directory: Some(dir.into()),
            };
            let episodes = PodConnection::new()
                .parse_feed(&pod, &HashMap::new(), 0)
                .await?;
            assert!(!episodes.is_empty());
            let query = postgres_query::query!(
                r#"
                    INSERT INTO podcasts (castid, castname, feedurl, directory)
                    VALUES ($castid,$castname,$feedurl,$directory)
                "#,
                castid = pod.castid,
                castname = pod.castname,
                feedurl = pod.feedurl,
                directory = pod.directory
            );
            pool.get()
                .await?
                .execute(query.sql(), &query.parameters())
                .await?;
            pod
        };
        Ok(pod)
    }

    pub async fn from_index(pool: &PgPool, cid: i32) -> Result<Option<Self>, Error> {
        let query = r#"
            SELECT
                castid, castname, feedurl, directory
            FROM podcasts
            WHERE castid = $1
        "#;
        if let Some(row) = pool.get().await?.query(query, &[&cid]).await?.get(0) {
            let pod = Self::from_row(row)?;
            Ok(Some(pod))
        } else {
            Ok(None)
        }
    }

    pub async fn from_feedurl(pool: &PgPool, feedurl: &str) -> Result<Option<Self>, Error> {
        let query = r#"
            SELECT
                castid, castname, feedurl, directory
            FROM podcasts
            WHERE feedurl = $1
        "#;
        if let Some(row) = pool
            .get()
            .await?
            .query(query, &[&feedurl.to_string()])
            .await?
            .get(0)
        {
            let pod = Self::from_row(row)?;
            Ok(Some(pod))
        } else {
            Ok(None)
        }
    }

    pub async fn get_all_podcasts(pool: &PgPool) -> Result<Vec<Self>, Error> {
        let query = r#"
            SELECT
                castid, castname, feedurl, directory
            FROM podcasts
        "#;
        pool.get()
            .await?
            .query(query, &[])
            .await?
            .iter()
            .map(|row| {
                let pod = Self::from_row(row)?;
                Ok(pod)
            })
            .collect()
    }

    pub async fn get_max_castid(pool: &PgPool) -> Result<i32, Error> {
        let query = "SELECT MAX(castid) FROM podcasts";
        match pool.get().await?.query(query, &[]).await?.get(0) {
            Some(row) => row.try_get(0).map_err(Into::into),
            None => Ok(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use log::debug;

    use crate::{config::Config, pgpool::PgPool, podcast::Podcast};

    #[tokio::test]
    #[ignore]
    async fn test_podcasts_from_index() {
        let config = Config::init_config().unwrap();
        let pool = PgPool::new(config.database_url.as_str());
        let p = Podcast::from_index(&pool, 19).await.unwrap().unwrap();
        debug!("{:?}", p);
        assert_eq!(
            p.castname.as_str(),
            "The Current Song of the Day - Minnesota Public Radio"
        );
        assert_eq!(
            p.feedurl.as_str(),
            "http://minnesota.publicradio.org/tools/podcasts/song-of-the-day.php"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_podcasts_from_feedurl() {
        let config = Config::init_config().unwrap();
        let pool = PgPool::new(config.database_url.as_str());
        let p = Podcast::from_feedurl(&pool, "http://nightvale.libsyn.com/rss")
            .await
            .unwrap()
            .unwrap();
        debug!("{:?}", p);
        assert_eq!(p.castid, 24);
        assert_eq!(p.castname.as_str(), "Welcome to Night Vale");
    }
}
