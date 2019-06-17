use failure::{err_msg, Error};
use std::fmt;
use std::str::FromStr;

use crate::exponential_retry::ExponentialRetry;
use crate::map_result_vec;
use crate::pgpool::PgPool;
use crate::pod_connection::PodConnection;
use crate::row_index_trait::RowIndexTrait;

#[derive(Clone, Copy, Debug)]
pub enum EpisodeStatus {
    Ready,
    Downloaded,
    Error,
    Skipped,
}

impl fmt::Display for EpisodeStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            EpisodeStatus::Ready => "Ready",
            EpisodeStatus::Downloaded => "Downloaded",
            EpisodeStatus::Error => "Error",
            EpisodeStatus::Skipped => "Skipped",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for EpisodeStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Ready" => Ok(EpisodeStatus::Ready),
            "Downloaded" => Ok(EpisodeStatus::Downloaded),
            "Error" => Ok(EpisodeStatus::Error),
            "Skipped" => Ok(EpisodeStatus::Skipped),
            _ => Err(err_msg(format!("Invalid string {}", s))),
        }
    }
}

impl Default for EpisodeStatus {
    fn default() -> Self {
        EpisodeStatus::Ready
    }
}

#[derive(Default, Clone, Debug)]
pub struct Episode {
    pub castid: i32,
    pub episodeid: i32,
    pub title: String,
    pub epurl: String,
    pub enctype: String,
    pub status: EpisodeStatus,
    pub eplength: i32,
    pub epfirstattempt: Option<i32>,
    pub eplastattempt: Option<i32>,
    pub epfailedattempts: i32,
    pub epguid: Option<String>,
}

impl Episode {
    pub fn get_all_episodes(pool: &PgPool, cid: i32) -> Result<Vec<Episode>, Error> {
        let query = r#"
            SELECT
                castid, episodeid, title, epurl, enctype, status, eplength, epfirstattempt,
                eplastattempt, epfailedattempts, epguid
            FROM episodes
            WHERE castid = $1
        "#;
        let results: Vec<Result<_, Error>> = pool
            .get()?
            .query(query, &[&cid])?
            .iter()
            .map(|row| {
                let castid: i32 = row.get_idx(0)?;
                let episodeid: i32 = row.get_idx(1)?;
                let title: String = row.get_idx(2)?;
                let epurl: String = row.get_idx(3)?;
                let enctype: String = row.get_idx(4)?;
                let status: String = row.get_idx(5)?;
                let eplength: i32 = row.get_idx(6)?;
                let epfirstattempt: Option<i32> = row.get_idx(7)?;
                let eplastattempt: Option<i32> = row.get_idx(8)?;
                let epfailedattempts: i32 = row.get_idx(9)?;
                let epguid: Option<String> = row.get_idx(10)?;

                let ep = Episode {
                    castid,
                    episodeid,
                    title,
                    epurl,
                    enctype,
                    status: status.parse()?,
                    eplength,
                    epfirstattempt,
                    eplastattempt,
                    epfailedattempts,
                    epguid,
                };
                Ok(ep)
            })
            .collect();

        map_result_vec(results)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::episode::Episode;
    use crate::pgpool::PgPool;

    #[test]
    fn test_episodes_get_all_episodes() {
        let config = Config::new().init_config().unwrap();
        let pool = PgPool::new(&config.database_url);

        let eps = Episode::get_all_episodes(&pool, 1).unwrap();

        assert!(eps.len() > 100);
    }
}
