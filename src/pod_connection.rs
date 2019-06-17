use failure::Error;
use reqwest::Client;
use roxmltree::{Document, NodeType};
use std::collections::HashMap;
use url::Url;

use crate::episode::Episode;
use crate::exponential_retry::ExponentialRetry;
use crate::podcast::Podcast;

pub struct PodConnection {
    client: Client,
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
        title: &Option<String>,
        epurl: &Option<String>,
        eplength: &Option<i32>,
        enctype: &Option<String>,
        filter_urls: &HashMap<Url, Episode>,
        latest_epid: i32,
    ) -> Option<Episode> {
        if let Some(epurl) = epurl.as_ref() {
            let url_exists = if let Ok(url) = epurl.parse() {
                filter_urls.contains_key(&url)
            } else {
                false
            };
            if !url_exists {
                let p = Episode {
                    title: title.clone().unwrap_or_else(|| "Unknown".to_string()),
                    castid: podcast.castid,
                    episodeid: latest_epid,
                    epurl: epurl.clone(),
                    eplength: eplength.unwrap_or(-1),
                    enctype: enctype.clone().unwrap_or_else(|| "".to_string()),
                    ..Default::default()
                };
                println!("new episode {:?}", p);
                return Some(p);
            } else {
                if let Ok(url) = epurl.parse() {
                    if let Some(epi) = filter_urls.get(&url) {
                        if let Some(title_) = title {
                            if &epi.title != title_ {
                                let mut p = epi.clone();
                                p.title = title_.clone();
                                println!("modified episode {:?}", p);
                                return Some(p);
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
        filter_urls: &HashMap<Url, Episode>,
        mut latest_epid: i32,
    ) -> Result<Vec<Episode>, Error> {
        let url = podcast.feedurl.parse()?;
        let text = self.get(&url)?.text()?;
        let doc = Document::parse(&text)?;

        let mut episodes = Vec::new();
        let mut title: Option<String> = None;
        let mut epurl: Option<String> = None;
        let mut eplength: Option<i32> = None;
        let mut enctype: Option<String> = None;

        for d in doc.root().descendants() {
            if d.node_type() == NodeType::Element && d.tag_name().name() == "title" {
                if epurl.is_some() {
                    if let Some(epi) = self.get_current_episode(
                        &podcast,
                        &title,
                        &epurl,
                        &eplength,
                        &enctype,
                        &filter_urls,
                        latest_epid,
                    ) {
                        episodes.push(epi);
                    }
                    title = None;
                    epurl = None;
                    eplength = None;
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
                    "length" => eplength = Some(a.value().parse()?),
                    "type" => enctype = Some(a.value().to_string()),
                    _ => (),
                }
            }
        }

        if let Some(epi) = self.get_current_episode(
            &podcast,
            &title,
            &epurl,
            &eplength,
            &enctype,
            &filter_urls,
            latest_epid,
        ) {
            episodes.push(epi);
        }

        Ok(episodes)
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
        let current_urls: HashMap<Url, Episode> = current_episodes
            .into_iter()
            .map(|e| (e.epurl.parse().unwrap(), e))
            .collect();

        let pod = Podcast::from_index(&pool, 1).unwrap().unwrap();
        let conn = PodConnection::new();
        let new_episodes = conn.parse_feed(&pod, &current_urls, max_epid + 1).unwrap();
        assert!(new_episodes.len() > 0);
        assert!(false);
    }
}
