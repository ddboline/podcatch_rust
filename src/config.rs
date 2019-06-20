use failure::{err_msg, Error};
use std::env::var;
use std::path::Path;

#[derive(Default, Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub google_music_directory: String,
}

impl Config {
    pub fn new() -> Config {
        Default::default()
    }

    pub fn from_env(mut self) -> Config {
        if let Ok(database_url) = var("DATABASE_URL") {
            self.database_url = database_url.to_string();
        }
        if let Ok(google_music_directory) = var("GOOGLE_MUSIC_DIRECTORY") {
            self.google_music_directory = google_music_directory.to_string();
        }
        self
    }

    pub fn init_config(self) -> Result<Config, Error> {
        let home_dir = var("HOME").map_err(|e| err_msg(format!("No HOME Directory {}", e)))?;

        let env_file = format!("{}/.config/podcatch_rust/config.env", home_dir);

        dotenv::dotenv().ok();

        if Path::new(&env_file).exists() {
            dotenv::from_path(&env_file).ok();
        } else if Path::new("config.env").exists() {
            dotenv::from_filename("config.env").ok();
        }

        Ok(self.from_env())
    }
}
