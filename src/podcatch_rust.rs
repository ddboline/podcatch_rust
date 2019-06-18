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

        update_episodes
            .into_par_iter()
            .map(|epi| {
                if let Ok(url) = epi.url_basename() {
                    if let Some(epguid) = epi.epguid.as_ref() {
                        if epguid.len() != 32 {
                            if let Some(directory) = pod.directory.as_ref() {
                                let fname = format!("{}/{}", directory, url);
                                let path = Path::new(&fname);
                                if path.exists() {
                                    if let Ok(md5sum) = get_md5sum(&path) {
                                        let mut p = epi.clone();
                                        println!("{} {}", fname, md5sum);
                                        p.epguid = Some(md5sum);
                                        p.update_episode(&pool).unwrap();
                                    }
                                } else {
                                    if let Ok(url_) = epi.epurl.parse::<Url>() {
                                        println!("download {:?} {}", url_, fname);
                                        // pod_conn.dump_to_file(&url_, &fname).unwrap();
                                        // let path = Path::new(&fname);
                                        // if path.exists() {
                                        //     if let Ok(md5sum) = get_md5sum(&path) {
                                        //         let mut p = epi.clone();
                                        //         println!("{} {}", fname, md5sum);
                                        //         p.epguid = Some(md5sum);
                                        //         p.update_episode(&pool).unwrap();
                                        //     }
                                        // }
                                    }
                                }
                            }
                        }
                    }
                }
                // epi.update_episode(&pool)?;
            })
            .for_each(drop);
    }
    Ok(())
}
