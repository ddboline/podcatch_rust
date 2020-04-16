use anyhow::{format_err, Error};
use futures::StreamExt;
use reqwest::{Client, Url};
use roxmltree::{Document, NodeType};
use std::{collections::HashMap, path::Path};
use tokio::{fs::File, io::AsyncWriteExt};

use crate::{
    episode::Episode, exponential_retry::ExponentialRetry, podcast::Podcast,
    stack_string::StackString,
};

#[derive(Clone)]
pub struct PodConnection {
    client: Client,
}

impl Default for PodConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl PodConnection {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    async fn get_current_episode(
        podcast: &Podcast,
        title: Option<&str>,
        epurl: Option<&str>,
        enctype: Option<&str>,
        filter_urls: &HashMap<StackString, Episode>,
        latest_epid: i32,
    ) -> Option<Episode> {
        if let Some(epurl) = epurl.as_ref() {
            let ep = Episode {
                title: title.map_or_else(|| "Unknown".into(), Into::into),
                castid: podcast.castid,
                episodeid: latest_epid,
                epurl: (*epurl).into(),
                enctype: enctype.map_or_else(|| "".into(), Into::into),
                ..Episode::default()
            };

            let url_exists = filter_urls.contains_key(&ep.title);

            if !url_exists {
                return Some(ep);
            } else if let Some(epi) = filter_urls.get(&ep.title) {
                if let Some(title_) = title {
                    if title_ == "Wedgie diplomacy: Bugle 4083" {
                        return None;
                    }
                    if &epi.title != title_ {
                        let mut p = epi.clone();
                        p.title = title_.into();
                        return Some(p);
                    } else if let Some(epguid) = epi.epguid.as_ref() {
                        if epguid.len() != 32 {
                            return Some(epi.clone());
                        }
                    }
                }
            }
        }
        None
    }

    pub async fn parse_feed(
        &self,
        podcast: &Podcast,
        filter_urls: &HashMap<StackString, Episode>,
        mut latest_epid: i32,
    ) -> Result<Vec<Episode>, Error> {
        let url = podcast.feedurl.parse()?;
        let text = self.get(&url).await?.text().await?;
        let doc = Document::parse(&text)?;

        let mut episodes = Vec::new();
        let mut title: Option<StackString> = None;
        let mut epurl: Option<StackString> = None;
        let mut enctype: Option<StackString> = None;

        for d in doc.root().descendants() {
            if d.node_type() == NodeType::Element && d.tag_name().name() == "title" {
                if epurl.is_some() {
                    if let Some(epi) = Self::get_current_episode(
                        &podcast,
                        title.as_ref().map(StackString::as_str),
                        epurl.as_ref().map(StackString::as_str),
                        enctype.as_ref().map(StackString::as_str),
                        &filter_urls,
                        latest_epid,
                    )
                    .await
                    {
                        episodes.push(epi);
                    }
                    title = None;
                    epurl = None;
                    enctype = None;
                    latest_epid += 1;
                }
                if let Some(t) = d.text() {
                    title = Some(t.into());
                }
            }
            for a in d.attributes() {
                match a.name() {
                    "url" => epurl = Some(a.value().into()),
                    "type" => enctype = Some(a.value().into()),
                    _ => (),
                }
            }
        }

        if let Some(epi) = Self::get_current_episode(
            &podcast,
            title.as_ref().map(StackString::as_str),
            epurl.as_ref().map(StackString::as_str),
            enctype.as_ref().map(StackString::as_str),
            &filter_urls,
            latest_epid,
        )
        .await
        {
            episodes.push(epi);
        }

        Ok(episodes)
    }

    pub async fn dump_to_file(&self, url: &Url, outpath: &Path) -> Result<(), Error> {
        if outpath.exists() {
            Err(format_err!("File exists"))
        } else {
            let mut f = File::create(outpath).await?;
            let mut byte_stream = self.get(url).await?.bytes_stream();
            while let Some(item) = byte_stream.next().await {
                f.write_all(&item?).await?;
            }
            Ok(())
        }
    }
}

impl ExponentialRetry for PodConnection {
    fn get_client(&self) -> &Client {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Url;
    use std::{collections::HashMap, fs::remove_file, path::Path};

    use crate::{
        config::Config, episode::Episode, exponential_retry::ExponentialRetry, pgpool::PgPool,
        pod_connection::PodConnection, podcast::Podcast, stack_string::StackString,
    };

    #[tokio::test]
    #[ignore]
    async fn test_pod_connection_get() {
        let config = Config::init_config().unwrap();
        let pool = PgPool::new(&config.database_url);
        let pod = Podcast::from_index(&pool, 1).await.unwrap().unwrap();
        let url: Url = pod.feedurl.parse().unwrap();
        let conn = PodConnection::new();
        let resp = conn.get(&url).await.unwrap();
        let text = resp.text().await.unwrap();

        assert!(text.starts_with("<?xml"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_pod_connection_parse_feed() {
        let config = Config::init_config().unwrap();
        let pool = PgPool::new(&config.database_url);
        let current_episodes = Episode::get_all_episodes(&pool, 1).await.unwrap();
        let max_epid = current_episodes
            .iter()
            .map(|e| e.episodeid)
            .max()
            .unwrap_or(0);
        let current_urls: HashMap<StackString, Episode> = current_episodes
            .into_iter()
            .map(|e| {
                let basename = e.url_basename().unwrap();

                (basename.into(), e)
            })
            .collect();

        let pod = Podcast::from_index(&pool, 23).await.unwrap().unwrap();
        let conn = PodConnection::new();
        let new_episodes = conn
            .parse_feed(&pod, &current_urls, max_epid + 1)
            .await
            .unwrap();
        assert!(new_episodes.len() > 0);
    }

    #[tokio::test]
    async fn test_dump_to_file() {
        let url = "https://dts.podtrac.com/redirect.mp3/api.entale.co/download/47015acd-f383-416d-8934-344cd944bfab/6215e4ba-ea1a-43e6-8d76-3de84fa5f52e/media.mp3";
        let url: Url = url.parse().unwrap();
        let pod_conn = PodConnection::new();
        if pod_conn
            .dump_to_file(&url, &Path::new("/tmp/temp.mp3"))
            .await
            .is_ok()
        {
            remove_file("/tmp/temp.mp3").unwrap();
        }
        assert!(true);
    }
}
