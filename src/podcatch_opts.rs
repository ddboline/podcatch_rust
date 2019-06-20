use failure::{err_msg, Error};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::collections::HashMap;
use std::path::Path;
use structopt::StructOpt;
use url::Url;

use crate::config::Config;
use crate::episode::{Episode, EpisodeStatus};
use crate::google_music::run_google_music;
use crate::pgpool::PgPool;
use crate::pod_connection::PodConnection;
use crate::podcast::Podcast;
use crate::{get_md5sum, map_result};

#[derive(StructOpt, Debug)]
pub struct PodcatchOpts {
    #[structopt(short = "l", long = "list")]
    do_list: bool,
    #[structopt(short = "a", long = "add")]
    do_add: bool,
    #[structopt(short = "n", long = "name")]
    podcast_name: Option<String>,
    #[structopt(short = "u", long = "url", parse(try_from_str))]
    podcast_url: Option<Url>,
    #[structopt(short = "i", long = "castid")]
    castid: Option<i32>,
    #[structopt(short = "d", long = "directory")]
    directory: Option<String>,
    #[structopt(short = "g", long = "google-music")]
    do_google_music: bool,
    #[structopt(short = "f", long = "filename")]
    filename: Option<String>,
}

impl PodcatchOpts {
    pub fn process_args() -> Result<(), Error> {
        let opts = PodcatchOpts::from_args();

        let config = Config::new().init_config()?;
        let pool = PgPool::new(&config.database_url);

        if opts.do_google_music {
            run_google_music(
                &config,
                opts.filename.as_ref().map(String::as_str),
                opts.do_add,
            )?;
        } else if opts.do_list {
            if let Some(castid) = opts.castid {
                for eps in &Episode::get_all_episodes(&pool, castid)? {
                    println!("{:?}", eps);
                }
            } else {
                for pod in &Podcast::get_all_podcasts(&pool)? {
                    println!("{:?}", pod);
                }
            }
        } else if opts.do_add {
            if let Some(podcast_name) = opts.podcast_name.as_ref() {
                if let Some(podcast_url) = opts.podcast_url.as_ref() {
                    let castid = match opts.castid {
                        Some(c) => c,
                        None => Podcast::get_max_castid(&pool)?,
                    };
                    let directory = opts.directory.unwrap_or_else(|| {
                        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                        format!("{}/{}", home_dir, podcast_name)
                    });
                    println!("Add {} {:?} {}", podcast_name, podcast_url, castid);
                    Podcast::add_podcast(&pool, castid, podcast_name, podcast_url, &directory)?;
                }
            }
        } else {
            process_all_podcasts(&pool)?;
        }
        Ok(())
    }
}

fn process_all_podcasts(pool: &PgPool) -> Result<(), Error> {
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
