use anyhow::Error;
use async_trait::async_trait;
use rand::{
    distr::{Distribution, Uniform},
    rng as thread_rng,
};
use reqwest::{Client, Response, Url};
use std::{convert::TryFrom, time::Duration};
use tokio::time::sleep;

#[async_trait]
pub trait ExponentialRetry {
    fn get_client(&self) -> &Client;

    async fn get(&self, url: &Url) -> Result<Response, Error> {
        let mut timeout: f64 = 1.0;
        let range = Uniform::try_from(0..1000)?;
        loop {
            match self.get_client().get(url.clone()).send().await {
                Ok(resp) => return Ok(resp),
                Err(err) => {
                    sleep(Duration::from_millis((timeout * 1000.0) as u64)).await;
                    timeout *= 4.0 * f64::from(range.sample(&mut thread_rng())) / 1000.0;
                    if timeout >= 64.0 {
                        return Err(err.into());
                    }
                }
            }
        }
    }
}
