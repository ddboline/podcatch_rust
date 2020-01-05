use failure::{err_msg, Error};
use postgres_query::FromSqlRow;
use reqwest::Url;
use std::collections::HashMap;

use crate::pgpool::PgPool;
use crate::pod_connection::PodConnection;

#[derive(Default, Clone, Debug, FromSqlRow)]
pub struct Podcast {
    pub castid: i32,
    pub castname: String,
    pub feedurl: String,
    pub directory: Option<String>,
}

impl Podcast {
    pub fn add_podcast(
        pool: &PgPool,
        cid: i32,
        cname: &str,
        furl: &Url,
        dir: &str,
    ) -> Result<Podcast, Error> {
        let pod = if let Some(p) = Podcast::from_index(&pool, cid)? {
            p
        } else if let Some(p) = Podcast::from_feedurl(&pool, furl.as_str())? {
            p
        } else {
            let pod = Podcast {
                castid: cid,
                castname: cname.to_string(),
                feedurl: furl.to_string(),
                directory: Some(dir.to_string()),
            };
            let episodes = PodConnection::new().parse_feed(&pod, &HashMap::new(), 0)?;
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
            pool.get()?.execute(query.sql, &query.parameters)?;
            pod
        };
        Ok(pod)
    }

    pub fn from_index(pool: &PgPool, cid: i32) -> Result<Option<Podcast>, Error> {
        let query = r#"
            SELECT
                castid, castname, feedurl, directory
            FROM podcasts
            WHERE castid = $1
        "#;
        if let Some(row) = pool.get()?.query(query, &[&cid])?.get(0) {
            let pod = Podcast::from_row(row)?;
            Ok(Some(pod))
        } else {
            Ok(None)
        }
    }

    pub fn from_feedurl(pool: &PgPool, feedurl: &str) -> Result<Option<Podcast>, Error> {
        let query = r#"
            SELECT
                castid, castname, feedurl, directory
            FROM podcasts
            WHERE feedurl = $1
        "#;
        if let Some(row) = pool.get()?.query(query, &[&feedurl.to_string()])?.get(0) {
            let pod = Podcast::from_row(row)?;
            Ok(Some(pod))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_podcasts(pool: &PgPool) -> Result<Vec<Podcast>, Error> {
        let query = r#"
            SELECT
                castid, castname, feedurl, directory
            FROM podcasts
        "#;
        pool.get()?
            .query(query, &[])?
            .iter()
            .map(|row| {
                let pod = Podcast::from_row(row)?;
                Ok(pod)
            })
            .collect()
    }

    pub fn get_max_castid(pool: &PgPool) -> Result<i32, Error> {
        let query = "SELECT MAX(castid) FROM podcasts";
        match pool.get()?.query(query, &[])?.get(0) {
            Some(row) => row.try_get(0).map_err(err_msg),
            None => Ok(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::pgpool::PgPool;
    use crate::podcast::Podcast;

    #[test]
    #[ignore]
    fn test_podcasts_from_index() {
        let config = Config::init_config().unwrap();
        let pool = PgPool::new(&config.database_url);
        let p = Podcast::from_index(&pool, 19).unwrap().unwrap();
        println!("{:?}", p);
        assert_eq!(
            p.castname,
            "The Current Song of the Day - Minnesota Public Radio"
        );
        assert_eq!(
            p.feedurl,
            "http://minnesota.publicradio.org/tools/podcasts/song-of-the-day.php"
        );
    }

    #[test]
    #[ignore]
    fn test_podcasts_from_feedurl() {
        let config = Config::init_config().unwrap();
        let pool = PgPool::new(&config.database_url);
        let p = Podcast::from_feedurl(&pool, "http://nightvale.libsyn.com/rss")
            .unwrap()
            .unwrap();
        println!("{:?}", p);
        assert_eq!(p.castid, 24);
        assert_eq!(p.castname, "Welcome to Night Vale");
    }
}
