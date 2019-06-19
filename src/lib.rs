pub mod config;
pub mod episode;
pub mod exponential_retry;
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
    let mut errors: Vec<_> = Vec::new();
    let output: V = input
        .into_iter()
        .filter_map(|item| match item {
            Ok(i) => Some(i),
            Err(e) => {
                errors.push(format!("{}", e));
                None
            }
        })
        .collect();
    if !errors.is_empty() {
        Err(err_msg(errors.join("\n")))
    } else {
        Ok(output)
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
