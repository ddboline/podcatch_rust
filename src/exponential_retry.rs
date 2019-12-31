use failure::{format_err, Error};
use log::error;
use reqwest::blocking::{Client, Response};
use reqwest::Url;
use retry::{delay::jitter, delay::Exponential, retry};

pub trait ExponentialRetry {
    fn get_client(&self) -> &Client;

    fn get(&self, url: &Url) -> Result<Response, Error> {
        retry(
            Exponential::from_millis(2)
                .map(jitter)
                .map(|x| x * 500)
                .take(6),
            || {
                self.get_client().get(url.clone()).send().map_err(|e| {
                    error!("Got error {:?} , retrying", e);
                    e
                })
            },
        )
        .map_err(|e| format_err!("{:?}", e))
    }
}
