use failure::{err_msg, Error};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::collections::HashMap;
use std::path::Path;
use url::Url;

use podcatch_rust::config::Config;
use podcatch_rust::episode::{Episode, EpisodeStatus};
use podcatch_rust::pgpool::PgPool;
use podcatch_rust::pod_connection::PodConnection;
use podcatch_rust::podcast::Podcast;
use podcatch_rust::{get_md5sum, map_result};

fn main() -> Result<(), Error> {
    let config = Config::new().init_config()?;
    let pool = PgPool::new(&config.database_url);
    let podcasts = Podcast::get_all_podcasts(&pool)?;
    let pod_conn = PodConnection::new();
    for pod in &podcasts {
        let episodes = Episode::get_all_episodes(&pool, pod.castid)?;
        let max_epid = Episode::get_max_epid(&pool)?;

        let results: Vec<_> = episodes
            .into_iter()
            .map(|e| {
                let basename = e.url_basename()?;
                Ok((basename, e))
            })
            .collect();

        let episode_map: HashMap<String, Episode> = map_result(results)?;

        let episode_list = pod_conn.parse_feed(&pod, &episode_map, max_epid + 1)?;

        let new_episodes: Vec<_> = episode_list
            .iter()
            .filter(|e| e.status == EpisodeStatus::Ready)
            .collect();
        let update_episodes: Vec<_> = episode_list
            .iter()
            .filter(|e| e.status != EpisodeStatus::Ready)
            .collect();

        println!(
            "{} {} {} {} {}",
            pod.castname,
            max_epid,
            episode_map.len(),
            new_episodes.len(),
            update_episodes.len(),
        );

        let results: Vec<_> = new_episodes
            .into_par_iter()
            .map(|epi| {
                if let Some(directory) = pod.directory.as_ref() {
                    println!(
                        "new download {} {} {}",
                        epi.epurl,
                        directory,
                        epi.url_basename()?
                    );
                    if let Some(mut new_epi) = Episode::from_epurl(&pool, pod.castid, &epi.epurl)? {
                        println!("new title {}", epi.title);
                        new_epi.title = epi.title.clone();
                        new_epi.update_episode(&pool)?;
                    } else {
                        let new_epi = epi.download_episode(&pod_conn, directory)?;
                        if new_epi.epguid.is_some() {
                            new_epi.insert_episode(&pool)?;
                        } else {
                            println!("No md5sum? {:?}", new_epi);
                        }
                    }
                }
                Ok(())
            })
            .collect();

        let _: Vec<_> = map_result(results)?;

        let results: Vec<_> = update_episodes
            .into_par_iter()
            .map(|epi| {
                let url = epi.url_basename()?;
                let epguid = epi.epguid.as_ref().ok_or_else(|| err_msg("no md5sum"))?;
                if epguid.len() != 32 {
                    if let Some(directory) = pod.directory.as_ref() {
                        let fname = format!("{}/{}", directory, url);
                        let path = Path::new(&fname);
                        if path.exists() {
                            if let Ok(md5sum) = get_md5sum(&path) {
                                let mut p = epi.clone();
                                println!("update md5sum {} {}", fname, md5sum);
                                p.epguid = Some(md5sum);
                                p.update_episode(&pool)?;
                            }
                        } else {
                            if let Ok(url_) = epi.epurl.parse::<Url>() {
                                println!("download {:?} {}", url_, fname);
                                let new_epi = epi.download_episode(&pod_conn, directory)?;
                                new_epi.update_episode(&pool)?;
                            }
                        }
                    }
                } else {
                    println!("{:?}", epi);
                }
                Ok(())
            })
            .collect();
        let _: Vec<_> = map_result(results)?;
    }
    Ok(())
}
