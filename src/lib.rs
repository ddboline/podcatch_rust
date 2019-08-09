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
use failure::{err_msg, Error};
use std::fs::File;
use std::iter::FromIterator;
use std::path::Path;

pub fn map_result<T, U, V>(input: U) -> Result<V, Error>
where
    U: IntoIterator<Item = Result<T, Error>>,
    V: FromIterator<T>,
{
    let (output, errors): (Vec<_>, Vec<_>) = input.into_iter().partition(Result::is_ok);
    if !errors.is_empty() {
        let errors: Vec<_> = errors
            .into_iter()
            .filter_map(Result::err)
            .map(|x| x.to_string())
            .collect();
        Err(err_msg(errors.join("\n")))
    } else {
        Ok(output.into_iter().filter_map(Result::ok).collect())
    }
}

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
