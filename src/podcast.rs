use failure::Error;

use crate::map_result;
use crate::pgpool::PgPool;
use crate::row_index_trait::RowIndexTrait;

#[derive(Default, Clone, Debug)]
pub struct Podcast {
    pub castid: i32,
    pub castname: String,
    pub feedurl: String,
    pub pcenabled: i32,
    pub lastupdate: Option<i32>,
    pub lastattempt: Option<i32>,
    pub failedattempts: i32,
    pub directory: Option<String>,
}

impl Podcast {
    pub fn add_podcast(
        pool: &PgPool,
        cid: i32,
        cname: &str,
        furl: &str,
        dir: &str,
    ) -> Result<Podcast, Error> {
        let pod = if let Some(p) = Podcast::from_index(&pool, cid)? {
            p
        } else if let Some(p) = Podcast::from_feedurl(&pool, furl)? {
            p
        } else {
            Podcast {
                castid: cid,
                castname: cname.to_string(),
                feedurl: furl.to_string(),
                directory: Some(dir.to_string()),
                ..Default::default()
            }
        };
        Ok(pod)
    }

    pub fn from_index(pool: &PgPool, cid: i32) -> Result<Option<Podcast>, Error> {
        let query = r#"
            SELECT
                castid, castname, feedurl, pcenabled, lastupdate, lastattempt, failedattempts,
                directory
            FROM podcasts
            WHERE castid = $1
        "#;
        if let Some(row) = pool.get()?.query(query, &[&cid])?.iter().nth(0) {
            let castid: i32 = row.get_idx(0)?;
            let castname: String = row.get_idx(1)?;
            let feedurl: String = row.get_idx(2)?;
            let pcenabled: i32 = row.get_idx(3)?;
            let lastupdate: Option<i32> = row.get_idx(4)?;
            let lastattempt: Option<i32> = row.get_idx(5)?;
            let failedattempts: i32 = row.get_idx(6)?;
            let directory: Option<String> = row.get_idx(7)?;

            let pod = Podcast {
                castid,
                castname,
                feedurl,
                pcenabled,
                lastupdate,
                lastattempt,
                failedattempts,
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
                castid, castname, feedurl, pcenabled, lastupdate, lastattempt, failedattempts,
                directory
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
            let pcenabled: i32 = row.get_idx(3)?;
            let lastupdate: Option<i32> = row.get_idx(4)?;
            let lastattempt: Option<i32> = row.get_idx(5)?;
            let failedattempts: i32 = row.get_idx(6)?;
            let directory: Option<String> = row.get_idx(7)?;

            let pod = Podcast {
                castid,
                castname,
                feedurl,
                pcenabled,
                lastupdate,
                lastattempt,
                failedattempts,
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
                castid, castname, feedurl, pcenabled, lastupdate, lastattempt, failedattempts,
                directory
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
                let pcenabled: i32 = row.get_idx(3)?;
                let lastupdate: Option<i32> = row.get_idx(4)?;
                let lastattempt: Option<i32> = row.get_idx(5)?;
                let failedattempts: i32 = row.get_idx(6)?;
                let directory: Option<String> = row.get_idx(7)?;

                let pod = Podcast {
                    castid,
                    castname,
                    feedurl,
                    pcenabled,
                    lastupdate,
                    lastattempt,
                    failedattempts,
                    directory,
                };
                Ok(pod)
            })
            .collect();

        map_result(results)
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
