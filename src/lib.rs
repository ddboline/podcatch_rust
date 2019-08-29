#[macro_use]
extern crate serde_derive;

pub mod config;
pub mod episode;
pub mod exponential_retry;
pub mod google_music;
pub mod pgpool;
pub mod pod_connection;
pub mod podcast;
pub mod podcatch_opts;
pub mod row_index_trait;

use checksums::{hash_file, Algorithm};
use failure::Error;
use std::fs::File;
use std::path::Path;

pub fn get_md5sum(path: &Path) -> Result<String, Error> {
    {
        File::open(path)?;
    }
    Ok(hash_file(path, Algorithm::MD5).to_lowercase())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
