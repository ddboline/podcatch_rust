#![allow(clippy::too_many_lines)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::similar_names)]
#![allow(clippy::shadow_unrelated)]
#![allow(clippy::used_underscore_binding)]
#![allow(clippy::missing_panics_doc)]

pub mod config;
pub mod episode;
pub mod episode_status;
pub mod exponential_retry;
pub mod pgpool;
pub mod pod_connection;
pub mod podcast;
pub mod podcatch_opts;

use anyhow::Error;
use checksums::{hash_reader, Algorithm};
use stack_string::StackString;
use std::{fs::File, path::Path};

/// # Errors
/// Return error if opening file fails
pub fn get_md5sum(path: &Path) -> Result<StackString, Error> {
    let mut f = File::open(path)?;
    Ok(hash_reader(&mut f, Algorithm::MD5).to_lowercase().into())
}
