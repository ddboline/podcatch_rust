use failure::{err_msg, Error};
use reqwest::{Client, Url};
use roxmltree::{Document, NodeType};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use crate::episode::Episode;
use crate::exponential_retry::ExponentialRetry;
use crate::podcast::Podcast;

pub struct PodConnection {
    client: Client,
}

impl Default for PodConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl PodConnection {
    pub fn new() -> PodConnection {
        Self {
            client: Client::new(),
        }
    }

    fn get_current_episode(
        &self,
        podcast: &Podcast,
        title: Option<&String>,
        epurl: Option<&String>,
        enctype: Option<&String>,
        filter_urls: &HashMap<String, Episode>,
        latest_epid: i32,
    ) -> Option<Episode> {
        if let Some(epurl) = epurl.as_ref() {
            let ep = Episode {
                title: title
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown".to_string()),
                castid: podcast.castid,
                episodeid: latest_epid,
                epurl: epurl.to_string(),
                enctype: enctype
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "".to_string()),
                ..Default::default()
            };

            let url_exists = if let Ok(url) = ep.url_basename() {
                filter_urls.contains_key(&url)
            } else {
                false
            };

            if !url_exists {
                return Some(ep);
            } else if let Ok(url) = ep.url_basename() {
                if let Some(epi) = filter_urls.get(&url) {
                    if let Some(title_) = title {
                        if title_ == "Wedgie diplomacy: Bugle 4083" {
                            return None;
                        }
                        if &epi.title != title_ {
                            let mut p = epi.clone();
                            p.title = title_.to_string();
                            return Some(p);
                        } else if let Some(epguid) = epi.epguid.as_ref() {
                            if epguid.len() != 32 {
                                return Some(epi.clone());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn parse_feed(
        &self,
        podcast: &Podcast,
        filter_urls: &HashMap<String, Episode>,
        mut latest_epid: i32,
    ) -> Result<Vec<Episode>, Error> {
        let url = podcast.feedurl.parse()?;
        let text = self.get(&url)?.text()?;
        let doc = Document::parse(&text)?;

        let mut episodes = Vec::new();
        let mut title: Option<String> = None;
        let mut epurl: Option<String> = None;
        let mut enctype: Option<String> = None;

        for d in doc.root().descendants() {
            if d.node_type() == NodeType::Element && d.tag_name().name() == "title" {
                if epurl.is_some() {
                    if let Some(epi) = self.get_current_episode(
                        &podcast,
                        title.as_ref(),
                        epurl.as_ref(),
                        enctype.as_ref(),
                        &filter_urls,
                        latest_epid,
                    ) {
                        episodes.push(epi);
                    }
                    title = None;
                    epurl = None;
                    enctype = None;
                    latest_epid += 1;
                }
                if let Some(t) = d.text() {
                    title = Some(t.to_string());
                }
            }
            for a in d.attributes() {
                match a.name() {
                    "url" => epurl = Some(a.value().to_string()),
                    "type" => enctype = Some(a.value().to_string()),
                    _ => (),
                }
            }
        }

        if let Some(epi) = self.get_current_episode(
            &podcast,
            title.as_ref(),
            epurl.as_ref(),
            enctype.as_ref(),
            &filter_urls,
            latest_epid,
        ) {
            episodes.push(epi);
        }

        Ok(episodes)
    }

    pub fn dump_to_file(&self, url: &Url, outfile: &str) -> Result<(), Error> {
        let outpath = Path::new(outfile);
        if outpath.exists() {
            Err(err_msg("File exists"))
        } else {
            let mut f = File::create(outfile)?;
            self.get(url)?.copy_to(&mut f)?;
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
    use std::collections::HashMap;
    use std::fs::remove_file;
    use url::Url;

    use crate::config::Config;
    use crate::episode::Episode;
    use crate::exponential_retry::ExponentialRetry;
    use crate::pgpool::PgPool;
    use crate::pod_connection::PodConnection;
    use crate::podcast::Podcast;

    #[test]
    fn test_pod_connection_get() {
        let config = Config::new().init_config().unwrap();
        let pool = PgPool::new(&config.database_url);
        let pod = Podcast::from_index(&pool, 1).unwrap().unwrap();
        let url: Url = pod.feedurl.parse().unwrap();
        let conn = PodConnection::new();
        let mut resp = conn.get(&url).unwrap();
        let text = resp.text().unwrap();

        assert!(text.starts_with("<?xml"));
    }

    #[test]
    fn test_pod_connection_parse_feed() {
        let config = Config::new().init_config().unwrap();
        let pool = PgPool::new(&config.database_url);
        let current_episodes = Episode::get_all_episodes(&pool, 1).unwrap();
        let max_epid = current_episodes
            .iter()
            .map(|e| e.episodeid)
            .max()
            .unwrap_or(0);
        let current_urls: HashMap<String, Episode> = current_episodes
            .into_iter()
            .map(|e| {
                let basename = e.url_basename().unwrap();

                (basename, e)
            })
            .collect();

        let pod = Podcast::from_index(&pool, 23).unwrap().unwrap();
        let conn = PodConnection::new();
        let new_episodes = conn.parse_feed(&pod, &current_urls, max_epid + 1).unwrap();
        assert!(new_episodes.len() > 0);
    }

    #[test]
    fn test_dump_to_file() {
        let url = "https://dts.podtrac.com/redirect.mp3/api.entale.co/download/47015acd-f383-416d-8934-344cd944bfab/6215e4ba-ea1a-43e6-8d76-3de84fa5f52e/media.mp3";
        let url: Url = url.parse().unwrap();
        let pod_conn = PodConnection::new();
        if pod_conn.dump_to_file(&url, "/tmp/temp.mp3").is_ok() {
            remove_file("/tmp/temp.mp3").unwrap();
        }
        assert!(true);
    }
}
