use anyhow::{format_err, Error};
use std::{env::var, ops::Deref, path::Path, sync::Arc};

#[derive(Default, Debug)]
pub struct ConfigInner {
    pub database_url: String,
    pub google_music_directory: String,
    pub user: String,
}

#[derive(Default, Debug, Clone)]
pub struct Config(Arc<ConfigInner>);

macro_rules! set_config {
    ($s:ident, $id:ident) => {
        if let Ok($id) = var(&stringify!($id).to_uppercase()) {
            $s.$id = $id;
        }
    };
}

impl ConfigInner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_env() -> Self {
        let mut conf = Self::default();

        set_config!(conf, database_url);
        set_config!(conf, google_music_directory);
        set_config!(conf, user);

        conf
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init_config() -> Result<Self, Error> {
        let fname = Path::new("config.env");
        let config_dir = dirs::config_dir().ok_or_else(|| format_err!("No CONFIG directory"))?;
        let default_fname = config_dir.join("podcatch_rust").join("config.env");

        let env_file = if fname.exists() {
            fname
        } else {
            &default_fname
        };

        dotenv::dotenv().ok();

        if env_file.exists() {
            dotenv::from_path(env_file).ok();
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
