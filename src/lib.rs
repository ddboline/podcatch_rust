pub mod config;
pub mod episode;
pub mod pgpool;
pub mod podcast;

use failure::{err_msg, Error};

pub fn map_result_vec<T>(input: Vec<Result<T, Error>>) -> Result<Vec<T>, Error> {
    let mut errors: Vec<_> = Vec::new();
    let output: Vec<T> = input
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
