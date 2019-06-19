use failure::Error;
use std::collections::HashMap;
use url::Url;

use crate::map_result;
use crate::pgpool::PgPool;
use crate::pod_connection::PodConnection;
use crate::row_index_trait::RowIndexTrait;

#[derive(Default, Clone, Debug)]
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
                ..Default::default()
            };
            let episodes = PodConnection::new().parse_feed(&pod, &HashMap::new(), 0)?;
            assert!(episodes.len() > 0);
            let query = r#"
                INSERT INTO podcasts (castid, castname, feedurl, directory)
                VALUES ($1,$2,$3,$4)
            "#;
            pool.get()?.execute(
                query,
                &[&pod.castid, &pod.castname, &pod.feedurl, &pod.directory],
            )?;
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
        if let Some(row) = pool.get()?.query(query, &[&cid])?.iter().nth(0) {
            let castid: i32 = row.get_idx(0)?;
            let castname: String = row.get_idx(1)?;
            let feedurl: String = row.get_idx(2)?;
            let directory: Option<String> = row.get_idx(3)?;

            let pod = Podcast {
                castid,
                castname,
                feedurl,
                directory,
            };
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
        if let Some(row) = pool
            .get()?
            .query(query, &[&feedurl.to_string()])?
            .iter()
            .nth(0)
        {
            let castid: i32 = row.get_idx(0)?;
            let castname: String = row.get_idx(1)?;
            let feedurl: String = row.get_idx(2)?;
            let directory: Option<String> = row.get_idx(3)?;

            let pod = Podcast {
                castid,
                castname,
                feedurl,
                directory,
            };
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
        let results: Vec<Result<_, Error>> = pool
            .get()?
            .query(query, &[])?
            .iter()
            .map(|row| {
                let castid: i32 = row.get_idx(0)?;
                let castname: String = row.get_idx(1)?;
                let feedurl: String = row.get_idx(2)?;
                let directory: Option<String> = row.get_idx(3)?;

                let pod = Podcast {
                    castid,
                    castname,
                    feedurl,
                    directory,
                };
                Ok(pod)
            })
            .collect();

        map_result(results)
    }

    pub fn get_max_castid(pool: &PgPool) -> Result<i32, Error> {
        let query = "SELECT MAX(castid) FROM podcasts";
        match pool.get()?.query(query, &[])?.iter().nth(0) {
            Some(row) => row.get_idx(0),
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
    fn test_podcasts_from_index() {
        let config = Config::new().init_config().unwrap();
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
    fn test_podcasts_from_feedurl() {
        let config = Config::new().init_config().unwrap();
        let pool = PgPool::new(&config.database_url);
        let p = Podcast::from_feedurl(&pool, "http://nightvale.libsyn.com/rss")
            .unwrap()
            .unwrap();
        println!("{:?}", p);
        assert_eq!(p.castid, 24);
        assert_eq!(p.castname, "Welcome to Night Vale");
    }
}
