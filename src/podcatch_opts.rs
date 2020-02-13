use anyhow::{format_err, Error};
use reqwest::Url;
use std::collections::HashMap;
use std::io::{stdout, Write};
use std::path::Path;
use structopt::StructOpt;
use tokio::task::spawn_blocking;

use crate::config::Config;
use crate::episode::Episode;
use crate::episode_status::EpisodeStatus;
use crate::get_md5sum;
use crate::google_music::{run_google_music, upload_list_of_mp3s, GoogleMusicMetadata};
use crate::pgpool::PgPool;
use crate::pod_connection::PodConnection;
use crate::podcast::Podcast;

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
    pub async fn process_args() -> Result<(), Error> {
        let opts = Self::from_args();

        let config = Config::init_config()?;
        let pool = PgPool::new(&config.database_url);

        if opts.do_google_music {
            let config_ = config.clone();
            let t = spawn_blocking(move || GoogleMusicMetadata::get_uploaded_mp3(&config_));
            process_all_podcasts(&pool, &config).await?;
            let mut metadata = t.await.expect("get_uploaded_mp3 paniced")?;
            run_google_music(
                &config,
                &mut metadata,
                opts.filename.as_ref().map(String::as_str),
                opts.do_add,
                &pool,
            )
            .await?;
        } else if opts.do_list {
            if let Some(castid) = opts.castid {
                for eps in &Episode::get_all_episodes(&pool, castid).await? {
                    writeln!(stdout().lock(), "{:?}", eps)?;
                }
            } else {
                for pod in &Podcast::get_all_podcasts(&pool).await? {
                    writeln!(stdout().lock(), "{:?}", pod)?;
                }
            }
        } else if opts.do_add {
            if let Some(podcast_name) = opts.podcast_name.as_ref() {
                if let Some(podcast_url) = opts.podcast_url.as_ref() {
                    let castid = match opts.castid {
                        Some(c) => c,
                        None => Podcast::get_max_castid(&pool).await?,
                    };
                    let directory = opts.directory.unwrap_or_else(|| {
                        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                        format!("{}/{}", home_dir, podcast_name)
                    });
                    writeln!(
                        stdout().lock(),
                        "Add {} {:?} {}",
                        podcast_name,
                        podcast_url,
                        castid
                    )?;
                    Podcast::add_podcast(&pool, castid, podcast_name, podcast_url, &directory).await?;
                }
            }
        } else {
            process_all_podcasts(&pool, &config).await?;
        }
        Ok(())
    }
}

async fn process_all_podcasts(pool: &PgPool, config: &Config) -> Result<(), Error> {
    let pod_conn = PodConnection::new();

    for pod in Podcast::get_all_podcasts(&pool).await? {
        let episodes = Episode::get_all_episodes(&pool, pod.castid).await?;
        let max_epid = Episode::get_max_epid(&pool).await?;

        let episode_map: Result<HashMap<String, Episode>, Error> = episodes
            .into_iter()
            .map(|e| {
                let basename = e.url_basename()?;
                Ok((basename, e))
            })
            .collect();

        let episode_map = episode_map?;

        let episode_list = pod_conn
            .parse_feed(&pod, &episode_map, max_epid + 1)
            .await?;

        let new_episodes: Vec<_> = episode_list
            .iter()
            .filter(|e| e.status == EpisodeStatus::Ready)
            .collect();
        let update_episodes: Vec<_> = episode_list
            .iter()
            .filter(|e| e.status != EpisodeStatus::Ready)
            .collect();

        let stdout = stdout();

        writeln!(
            stdout.lock(),
            "{} {} {} {} {}",
            pod.castname,
            max_epid,
            episode_map.len(),
            new_episodes.len(),
            update_episodes.len(),
        )?;

        for epi in new_episodes {
            if let Some(directory) = pod.directory.as_ref() {
                writeln!(
                    stdout.lock(),
                    "new download {} {} {}",
                    epi.epurl,
                    directory,
                    epi.url_basename()?
                )?;
                if let Some(mut new_epi) =
                    Episode::from_epurl(&pool, pod.castid, &epi.epurl).await?
                {
                    writeln!(stdout.lock(), "new title {}", epi.title)?;
                    new_epi.title = epi.title.to_string();
                    new_epi.update_episode(&pool).await?;
                } else {
                    let new_epi = epi.download_episode(&pod_conn, directory).await?;
                    if new_epi.epguid.is_some() {
                        new_epi.insert_episode(&pool).await?;
                        if directory.contains(&config.google_music_directory) {
                            let outfile = format!("{}/{}", directory, new_epi.url_basename()?);
                            let path = Path::new(&outfile);
                            if path.exists() {
                                let l = upload_list_of_mp3s(config, &[path.to_path_buf()])
                                    .map_err(|e| format_err!("{:?}", e))?;
                                writeln!(stdout.lock(), "ids {:?}", l)?;
                            }
                        }
                    } else {
                        writeln!(stdout.lock(), "No md5sum? {:?}", new_epi)?;
                    }
                }
            }
        }

        for epi in update_episodes {
            let url = epi.url_basename()?;
            let epguid = epi
                .epguid
                .as_ref()
                .ok_or_else(|| format_err!("no md5sum"))?;
            if epguid.len() == 32 {
                writeln!(stdout.lock(), "{:?}", epi)?;
            } else if let Some(directory) = pod.directory.as_ref() {
                let fname = format!("{}/{}", directory, url);
                let path = Path::new(&fname);
                if path.exists() {
                    if let Ok(md5sum) = get_md5sum(&path) {
                        let mut p = epi.clone();
                        writeln!(stdout.lock(), "update md5sum {} {}", fname, md5sum)?;
                        p.epguid = Some(md5sum);
                        p.update_episode(&pool).await?;
                    }
                } else if let Ok(url_) = epi.epurl.parse::<Url>() {
                    writeln!(stdout.lock(), "download {:?} {}", url_, fname)?;
                    let new_epi = epi.download_episode(&pod_conn, directory).await?;
                    new_epi.update_episode(&pool).await?;
                }
            }
        }
    }
    Ok(())
}
