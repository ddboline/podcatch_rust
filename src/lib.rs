#![allow(clippy::must_use_candidate)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::similar_names)]
#![allow(clippy::shadow_unrelated)]
#![allow(clippy::missing_errors_doc)]

pub mod config;
pub mod episode;
pub mod episode_status;
pub mod exponential_retry;
pub mod google_music;
pub mod pgpool;
pub mod pod_connection;
pub mod podcast;
pub mod podcatch_opts;
pub mod stdout_channel;

use anyhow::Error;
use checksums::{hash_file, Algorithm};
use std::{fs::File, path::Path};

pub fn get_md5sum(path: &Path) -> Result<String, Error> {
    {
        File::open(path)?;
    }
    Ok(hash_file(path, Algorithm::MD5).to_lowercase())
}
