use failure::{err_msg, Error};
use rand::distributions::{Distribution, Uniform};
use rand::thread_rng;
use reqwest::{Client, Response, Url};
use std::thread::sleep;
use std::time::Duration;

pub trait ExponentialRetry {
    fn get_client(&self) -> &Client;

    fn get(&self, url: &Url) -> Result<Response, Error> {
        let mut timeout: f64 = 1.0;
        let mut rng = thread_rng();
        let range = Uniform::from(0..1000);
        loop {
            match self.get_client().get(url.clone()).send() {
                Ok(x) => return Ok(x),
                Err(e) => {
                    sleep(Duration::from_millis((timeout * 1000.0) as u64));
                    timeout *= 4.0 * f64::from(range.sample(&mut rng)) / 1000.0;
                    if timeout >= 64.0 {
                        return Err(err_msg(e));
                    }
                }
            }
        }
    }
}
