use anyhow::{format_err, Error};
use futures::future::try_join_all;
use reqwest::Url;
use stack_string::StackString;
use std::{collections::HashSet, path::Path, sync::Arc};
use structopt::StructOpt;
use tokio::task::spawn_blocking;

use crate::{
    config::Config,
    episode::Episode,
    episode_status::EpisodeStatus,
    get_md5sum,
    google_music::{run_google_music, upload_list_of_mp3s, GoogleMusicMetadata},
    pgpool::PgPool,
    pod_connection::PodConnection,
    podcast::Podcast,
    stdout_channel::StdoutChannel,
};

#[derive(StructOpt, Debug)]
pub struct PodcatchOpts {
    #[structopt(short = "l", long = "list")]
    do_list: bool,
    #[structopt(short = "a", long = "add")]
    do_add: bool,
    #[structopt(short = "n", long = "name")]
    podcast_name: Option<StackString>,
    #[structopt(short = "u", long = "url", parse(try_from_str))]
    podcast_url: Option<Url>,
    #[structopt(short = "i", long = "castid")]
    castid: Option<i32>,
    #[structopt(short = "d", long = "directory")]
    directory: Option<StackString>,
    #[structopt(short = "g", long = "google-music")]
    do_google_music: bool,
    #[structopt(short = "f", long = "filename")]
    filename: Option<StackString>,
}

impl PodcatchOpts {
    pub async fn process_args() -> Result<(), Error> {
        let opts = Self::from_args();

        let config = Config::init_config()?;
        let pool = PgPool::new(&config.database_url);
        let stdout = StdoutChannel::new();
        let task = stdout.spawn_stdout_task();

        if opts.do_google_music {
            let metadata = {
                let config = config.clone();
                spawn_blocking(move || GoogleMusicMetadata::get_uploaded_mp3(&config))
            };

            process_all_podcasts(&pool, &config, &stdout).await?;

            let metadata = metadata.await.expect("get_uploaded_mp3 paniced")?;
            run_google_music(
                &config,
                metadata,
                opts.filename.as_ref().map(StackString::as_str),
                opts.do_add,
                &pool,
                &stdout,
            )
            .await?;
        } else if opts.do_list {
            if let Some(castid) = opts.castid {
                for eps in &Episode::get_all_episodes(&pool, castid).await? {
                    stdout.send(format!("{:?}", eps).into())?;
                }
            } else {
                for pod in &Podcast::get_all_podcasts(&pool).await? {
                    stdout.send(format!("{:?}", pod).into())?;
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
                        format!("{}/{}", home_dir, podcast_name).into()
                    });
                    stdout.send(
                        format!("Add {} {:?} {}", podcast_name, podcast_url, castid).into(),
                    )?;
                    Podcast::add_podcast(&pool, castid, &podcast_name, podcast_url, &directory)
                        .await?;
                }
            }
        } else {
            process_all_podcasts(&pool, &config, &stdout).await?;
        }
        stdout.close().await?;
        task.await?
    }
}

async fn process_all_podcasts(
    pool: &PgPool,
    config: &Config,
    stdout: &StdoutChannel,
) -> Result<(), Error> {
    let pod_conn = PodConnection::new();

    let futures: Vec<_> = Podcast::get_all_podcasts(&pool)
        .await?
        .into_iter()
        .map(|pod| {
            let pool = pool.clone();
            let pod_conn = pod_conn.clone();
            let pod = Arc::new(pod);
            async move {
                let episodes = Episode::get_all_episodes(&pool, pod.castid).await?;
                let max_epid = Episode::get_max_epid(&pool).await?;

                let episode_map: Result<HashSet<Episode>, Error> =
                    episodes.into_iter().map(Ok).collect();

                let episode_map = episode_map?;

                let episode_list = pod_conn
                    .parse_feed(&pod, &episode_map, max_epid + 1)
                    .await?;
                let episode_list = Arc::new(episode_list);

                Ok((pod, episode_list, max_epid, episode_map))
            }
        })
        .collect();
    let results: Result<Vec<_>, Error> = try_join_all(futures).await;

    for (pod, episode_list, max_epid, episode_map) in results? {
        let new_episodes: Vec<_> = episode_list
            .iter()
            .filter(|e| e.status == EpisodeStatus::Ready)
            .collect();
        let update_episodes: Vec<_> = episode_list
            .iter()
            .filter(|e| e.status != EpisodeStatus::Ready && e.status != EpisodeStatus::Downloaded)
            .collect();

        stdout.send(
            format!(
                "podcast {} {} {} {} {}",
                pod.castname,
                max_epid,
                episode_map.len(),
                new_episodes.len(),
                update_episodes.len(),
            )
            .into(),
        )?;

        let futures: Vec<_> = new_episodes
            .into_iter()
            .map(|epi| {
                let pod = pod.clone();
                let pod_conn = pod_conn.clone();
                async move {
                    if let Some(directory) = pod.directory.as_ref() {
                        let directory_path = Path::new(directory.as_str());
                        let mut output = vec![format!(
                            "new download {} {} {}",
                            epi.epurl,
                            directory,
                            epi.url_basename()?
                        )];
                        if let Some(mut new_epi) =
                            Episode::from_epurl(&pool, pod.castid, &epi.epurl).await?
                        {
                            output.push(format!("new title {}", epi.title));
                            new_epi.title = epi.title.clone();
                            new_epi.update_episode(&pool).await?;
                        } else {
                            let new_epi = epi.download_episode(&pod_conn, directory_path).await?;
                            if new_epi.epguid.is_some() {
                                new_epi.insert_episode(&pool).await?;
                                if directory.contains(config.google_music_directory.as_str()) {
                                    let outfile =
                                        format!("{}/{}", directory, new_epi.url_basename()?);
                                    let path = Path::new(&outfile);
                                    if path.exists() {
                                        let l = upload_list_of_mp3s(config, &[path.to_path_buf()])
                                            .map_err(|e| format_err!("{:?}", e))?;
                                        output.push(format!("ids {:?}", l));
                                    }
                                } else if directory.contains("The_Bugle") {
                                }
                            } else {
                                output.push(format!("No md5sum? {:?}", new_epi));
                            }
                        }
                        Ok(Some(output))
                    } else {
                        Ok(None)
                    }
                }
            })
            .collect();
        let results: Result<Vec<_>, Error> = try_join_all(futures).await;
        for line in results?.into_iter().filter_map(|x| x) {
            stdout.send(line.join("\n").into())?;
        }

        let futures: Vec<_> = update_episodes
            .into_iter()
            .map(|epi| {
                let pod = pod.clone();
                let pod_conn = pod_conn.clone();
                async move {
                    let mut output = Vec::new();
                    let url = epi.url_basename()?;
                    let epguid = epi
                        .epguid
                        .as_ref()
                        .ok_or_else(|| format_err!("no md5sum"))?;
                    if let Some(directory) = pod.directory.as_ref() {
                        let directory_path = Path::new(directory.as_str());
                        if epguid.len() != 32 {
                            let path = directory_path.join(url.as_str());
                            let fname = path.to_string_lossy();
                            if path.exists() {
                                if let Ok(md5sum) = get_md5sum(&path) {
                                    let mut p = epi.clone();
                                    output.push(format!("update md5sum {} {}", fname, md5sum));
                                    p.epguid = Some(md5sum.into());
                                    p.update_episode(&pool).await?;
                                }
                            } else if let Ok(url_) = epi.epurl.parse::<Url>() {
                                output.push(format!("download {:?} {}", url_, fname));
                                let new_epi =
                                    epi.download_episode(&pod_conn, directory_path).await?;
                                new_epi.update_episode(&pool).await?;
                            }
                        }
                    }
                    Ok(output)
                }
            })
            .collect();
        let results: Result<Vec<_>, Error> = try_join_all(futures).await;
        for line in results? {
            stdout.send(line.join("\n").into())?;
        }
    }
    Ok(())
}
