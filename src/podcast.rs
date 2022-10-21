use anyhow::Error;
use postgres_query::{query, FromSqlRow};
use reqwest::Url;
use stack_string::StackString;
use std::collections::HashSet;

use crate::{pgpool::PgPool, pod_connection::PodConnection};

#[derive(Default, Clone, Debug, FromSqlRow)]
pub struct Podcast {
    pub castid: i32,
    pub castname: StackString,
    pub feedurl: StackString,
    pub directory: Option<StackString>,
}

impl Podcast {
    /// # Errors
    /// Return error if db query fails
    pub async fn add_podcast(
        pool: &PgPool,
        cid: i32,
        cname: &str,
        furl: &Url,
        dir: &str,
    ) -> Result<Self, Error> {
        let pod = if let Some(p) = Self::from_index(pool, cid).await? {
            p
        } else if let Some(p) = Self::from_feedurl(pool, furl.as_str()).await? {
            p
        } else {
            let pod = Self {
                castid: cid,
                castname: cname.into(),
                feedurl: furl.as_str().into(),
                directory: Some(dir.into()),
            };
            let episodes = PodConnection::new()
                .parse_feed(&pod, &HashSet::new(), 0)
                .await?;
            assert!(!episodes.is_empty());
            let query = query!(
                r#"
                    INSERT INTO podcasts (castid, castname, feedurl, directory)
                    VALUES ($castid,$castname,$feedurl,$directory)
                "#,
                castid = pod.castid,
                castname = pod.castname,
                feedurl = pod.feedurl,
                directory = pod.directory
            );
            let conn = pool.get().await?;
            query.fetch_one(&conn).await?
        };
        Ok(pod)
    }

    /// # Errors
    /// Return error if db query fails
    pub async fn from_index(pool: &PgPool, cid: i32) -> Result<Option<Self>, Error> {
        let query = query!(
            r#"
                SELECT
                    castid, castname, feedurl, directory
                FROM podcasts
                WHERE castid = $castid
            "#,
            castid = cid
        );
        let conn = pool.get().await?;
        query.fetch_opt(&conn).await.map_err(Into::into)
    }

    /// # Errors
    /// Return error if db query fails
    pub async fn from_feedurl(pool: &PgPool, feedurl: &str) -> Result<Option<Self>, Error> {
        let query = query!(
            r#"
                SELECT
                    castid, castname, feedurl, directory
                FROM podcasts
                WHERE feedurl = $feedurl
            "#,
            feedurl = feedurl
        );
        let conn = pool.get().await?;
        query.fetch_opt(&conn).await.map_err(Into::into)
    }

    /// # Errors
    /// Return error if db query fails
    pub async fn get_all_podcasts(pool: &PgPool) -> Result<Vec<Self>, Error> {
        let query = query!(
            r#"
            SELECT
                castid, castname, feedurl, directory
            FROM podcasts
        "#
        );
        let conn = pool.get().await?;
        query.fetch(&conn).await.map_err(Into::into)
    }

    /// # Errors
    /// Return error if db query fails
    pub async fn get_max_castid(pool: &PgPool) -> Result<Option<i32>, Error> {
        #[derive(FromSqlRow)]
        struct Wrap(i32);

        let query = query!("SELECT MAX(castid) FROM podcasts");
        let conn = pool.get().await?;
        let val: Option<Wrap> = query.fetch_opt(&conn).await?;
        Ok(val.map(|x| x.0))
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
        let pool = PgPool::new(&config.database_url);
        let p = Podcast::from_index(&pool, 19).await.unwrap().unwrap();
        debug!("{:?}", p);
        assert_eq!(
            &p.castname,
            "The Current Song of the Day - Minnesota Public Radio"
        );
        assert_eq!(
            &p.feedurl,
            "http://minnesota.publicradio.org/tools/podcasts/song-of-the-day.php"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_podcasts_from_feedurl() {
        let config = Config::init_config().unwrap();
        let pool = PgPool::new(&config.database_url);
        let p = Podcast::from_feedurl(
            &pool,
            "http://feeds.nightvalepresents.com/welcometonightvalepodcast",
        )
        .await
        .unwrap()
        .unwrap();
        debug!("{:?}", p);
        assert_eq!(p.castid, 24);
        assert_eq!(&p.castname, "Welcome to Night Vale");
    }
}
