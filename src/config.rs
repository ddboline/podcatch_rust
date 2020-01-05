use failure::{format_err, Error};
use std::env::var;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct ConfigInner {
    pub database_url: String,
    pub google_music_directory: String,
    pub user: String,
}

#[derive(Default, Debug, Clone)]
pub struct Config(Arc<ConfigInner>);

impl ConfigInner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_env() -> Self {
        let mut conf = Self::default();
        if let Ok(database_url) = var("DATABASE_URL") {
            conf.database_url = database_url;
        }
        if let Ok(google_music_directory) = var("GOOGLE_MUSIC_DIRECTORY") {
            conf.google_music_directory = google_music_directory;
        }
        if let Ok(user) = var("USER") {
            conf.user = user;
        }
        conf
    }
}

impl Config {
    pub fn new() -> Self {
        Config::default()
    }

    pub fn init_config() -> Result<Self, Error> {
        let home_dir = var("HOME").map_err(|e| format_err!("No HOME Directory {}", e))?;

        let env_file = format!("{}/.config/podcatch_rust/config.env", home_dir);

        dotenv::dotenv().ok();

        if Path::new(&env_file).exists() {
            dotenv::from_path(&env_file).ok();
        } else if Path::new("config.env").exists() {
            dotenv::from_filename("config.env").ok();
        }

        let config = ConfigInner::from_env();

        Ok(Self(Arc::new(config)))
    }
}

impl Deref for Config {
    type Target = ConfigInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
