use anyhow::{format_err, Error};
use serde::Deserialize;
use std::{ops::Deref, path::Path, sync::Arc};

use stack_string::StackString;

#[derive(Default, Debug, Deserialize)]
pub struct ConfigInner {
    pub database_url: StackString,
    pub user: StackString,
}

#[derive(Default, Debug, Clone)]
pub struct Config(Arc<ConfigInner>);

impl ConfigInner {
    fn from_env() -> Self {
        envy::from_env().unwrap_or_else(|_| Self::default())
    }
}

impl Config {
    /// # Errors
    /// Return error if parsing environment variables fails
    pub fn init_config() -> Result<Self, Error> {
        let fname = Path::new("config.env");
        let config_dir = dirs::config_dir().ok_or_else(|| format_err!("No CONFIG directory"))?;
        let default_fname = config_dir.join("podcatch_rust").join("config.env");

        let env_file = if fname.exists() {
            fname
        } else {
            &default_fname
        };

        dotenvy::dotenv().ok();

        if env_file.exists() {
            dotenvy::from_path(env_file).ok();
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
