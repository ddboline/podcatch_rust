use failure::{err_msg, Error};
use std::env::var;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct ConfigInner {
    pub database_url: String,
    pub google_music_directory: String,
}

#[derive(Default, Debug, Clone)]
pub struct Config(Arc<ConfigInner>);

impl ConfigInner {
    pub fn new() -> ConfigInner {
        Default::default()
    }

    pub fn from_env() -> ConfigInner {
        let mut conf = ConfigInner::default();
        if let Ok(database_url) = var("DATABASE_URL") {
            conf.database_url = database_url.to_string();
        }
        if let Ok(google_music_directory) = var("GOOGLE_MUSIC_DIRECTORY") {
            conf.google_music_directory = google_music_directory.to_string();
        }
        conf
    }
}

impl Config {
    pub fn new() -> Config {
        Default::default()
    }

    pub fn init_config() -> Result<Config, Error> {
        let home_dir = var("HOME").map_err(|e| err_msg(format!("No HOME Directory {}", e)))?;

        let env_file = format!("{}/.config/podcatch_rust/config.env", home_dir);

        dotenv::dotenv().ok();

        if Path::new(&env_file).exists() {
            dotenv::from_path(&env_file).ok();
        } else if Path::new("config.env").exists() {
            dotenv::from_filename("config.env").ok();
        }

        let config = ConfigInner::from_env();

        Ok(Config(Arc::new(config)))
    }
}

impl Deref for Config {
    type Target = ConfigInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
