use failure::{err_msg, Error};
use std::fmt;
use std::fs::remove_file;
use std::path::Path;
use std::str::FromStr;
use url::Url;

use crate::pgpool::PgPool;
use crate::pod_connection::PodConnection;
use crate::row_index_trait::RowIndexTrait;
use crate::{get_md5sum, map_result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    pub fn url_basename(&self) -> Result<String, Error> {
        if self.epurl.ends_with("media.mp3") {
            let basename: String = self
                .title
                .to_lowercase()
                .chars()
                .filter_map(|c| match c {
                    'a'...'z' => Some(c),
                    '0'...'9' => Some(c),
                    ' ' => Some('_'),
                    _ => None,
                })
                .collect();
            Ok(format!("{}.mp3", basename))
        } else {
            let epurl: Url = self.epurl.parse()?;
            epurl
                .path()
                .split('/')
                .last()
                .map(|s| s.to_string())
                .ok_or_else(|| err_msg("No basename"))
        }
    }

    pub fn from_index(pool: &PgPool, cid: i32, eid: i32) -> Result<Option<Episode>, Error> {
        let query = r#"
            SELECT
                castid, episodeid, title, epurl, enctype, status, eplength, epfirstattempt,
                eplastattempt, epfailedattempts, epguid
            FROM episodes
            WHERE castid = $1 AND episodeid = $2
        "#;
        if let Some(row) = pool.get()?.query(query, &[&cid, &eid])?.iter().nth(0) {
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
            Ok(Some(ep))
        } else {
            Ok(None)
        }
    }

    pub fn from_epurl(pool: &PgPool, cid: i32, epurl: &str) -> Result<Option<Episode>, Error> {
        let query = r#"
            SELECT
                castid, episodeid, title, epurl, enctype, status, eplength, epfirstattempt,
                eplastattempt, epfailedattempts, epguid
            FROM episodes
            WHERE castid = $1 AND epurl = $2
        "#;
        if let Some(row) = pool.get()?.query(query, &[&cid, &epurl])?.iter().nth(0) {
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
            Ok(Some(ep))
        } else {
            Ok(None)
        }
    }

    pub fn from_epguid(pool: &PgPool, cid: i32, epguid: &str) -> Result<Option<Episode>, Error> {
        let query = r#"
            SELECT
                castid, episodeid, title, epurl, enctype, status, eplength, epfirstattempt,
                eplastattempt, epfailedattempts, epguid
            FROM episodes
            WHERE castid = $1 AND epguid = $2
        "#;
        if let Some(row) = pool.get()?.query(query, &[&cid, &epguid])?.iter().nth(0) {
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
            Ok(Some(ep))
        } else {
            Ok(None)
        }
    }

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

        map_result(results)
    }

    pub fn insert_episode(&self, pool: &PgPool) -> Result<u64, Error> {
        let query = r#"
            INSERT INTO episodes (
                castid, episodeid, title, epurl, enctype, status, eplength, epfirstattempt,
                eplastattempt, epfailedattempts, epguid
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
            )
        "#;
        pool.get()?
            .execute(
                query,
                &[
                    &self.castid,
                    &self.episodeid,
                    &self.title,
                    &self.epurl,
                    &self.enctype,
                    &self.status.to_string(),
                    &self.eplength,
                    &self.epfirstattempt,
                    &self.eplastattempt,
                    &self.epfailedattempts,
                    &self.epguid,
                ],
            )
            .map_err(err_msg)
    }

    pub fn update_episode(&self, pool: &PgPool) -> Result<u64, Error> {
        let query = r#"
            UPDATE episodes
            SET title=$3,epurl=$4,enctype=$5,status=$6,eplength=$7,epfirstattempt=$8,
                eplastattempt=$9,epfailedattempts=$10,epguid=$11
            WHERE castid=$1 AND episodeid=$2
        "#;
        pool.get()?
            .execute(
                query,
                &[
                    &self.castid,
                    &self.episodeid,
                    &self.title,
                    &self.epurl,
                    &self.enctype,
                    &self.status.to_string(),
                    &self.eplength,
                    &self.epfirstattempt,
                    &self.eplastattempt,
                    &self.epfailedattempts,
                    &self.epguid,
                ],
            )
            .map_err(err_msg)
    }

    pub fn get_max_epid(pool: &PgPool) -> Result<i32, Error> {
        let query = "SELECT MAX(episodeid) FROM episodes";
        pool.get()?
            .query(query, &[])?
            .iter()
            .nth(0)
            .ok_or_else(|| err_msg("No episodes"))
            .and_then(|row| row.get_idx(0))
    }

    pub fn download_episode(
        &self,
        conn: &PodConnection,
        directory: &str,
    ) -> Result<Episode, Error> {
        if !Path::new(directory).exists() {
            Err(err_msg(format!("No such directory {}", directory)))
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
                println!("{} {}", outfile, md5sum);
                p.epguid = Some(md5sum);
                p.status = EpisodeStatus::Downloaded;
                Ok(p)
            } else {
                Err(err_msg("Download failed"))
            }
        } else {
            Err(err_msg(format!("Unkown failure {:?}", self)))
        }
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
