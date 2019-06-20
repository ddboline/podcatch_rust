use failure::{err_msg, Error};
use id3::Tag;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use cpython::{
    FromPyObject, ObjectProtocol, PyDict, PyList, PyObject, PyResult, PyString, PyTuple, Python,
    PythonObject,
};

use crate::config::Config;
use crate::map_result;

#[derive(Deserialize, Debug, Clone)]
pub struct GoogleMusicMetadata {
    pub id: String,
    pub title: String,
    pub album: String,
    pub album_artist: Option<String>,
    pub artist: String,
    pub track_number: Option<u32>,
    pub track_size: i32,
    pub disc_number: Option<i32>,
    pub total_disc_count: Option<i32>,
}

pub fn get_uploaded_mp3() -> PyResult<Vec<String>> {
    let gil = Python::acquire_gil();
    let py = gil.python();
    let google_music = py.import("google_music")?;
    let json = py.import("json")?;
    let ddboline = PyString::new(py, "ddboline");
    let mm: PyObject = google_music.call(
        py,
        "MusicManager",
        PyTuple::new(py, &[ddboline.into_object()]),
        None,
    )?;
    let args = PyDict::new(py);
    args.set_item(py, "uploaded", true)?;
    args.set_item(py, "purchased", false)?;
    let uploaded: PyObject = mm.call_method(py, "songs", PyTuple::empty(py), Some(&args))?;
    let uploaded = PyList::extract(py, &uploaded)?;
    let mut results = Vec::new();
    for item in uploaded.iter(py) {
        let dict = PyDict::extract(py, &item)?;
        let js: PyObject = json.call(py, "dumps", PyTuple::new(py, &[dict.into_object()]), None)?;
        let js = PyString::extract(py, &js)?;
        let result = js.to_string(py)?;
        results.push(result.to_string());
    }
    Ok(results)
}

pub fn upload_list_of_mp3s(filelist: &[PathBuf]) -> PyResult<()> {
    let gil = Python::acquire_gil();
    let py = gil.python();
    let google_music = py.import("google_music")?;
    let ddboline = PyString::new(py, "ddboline");
    let mm: PyObject = google_music.call(
        py,
        "MusicManager",
        PyTuple::new(py, &[ddboline.into_object()]),
        None,
    )?;
    for p in filelist {
        if let Some(s) = p.to_str() {
            println!("upload {}", s);
            let fname = PyString::new(py, s);
            mm.call_method(py, "upload", PyTuple::new(py, &[fname.into_object()]), None)?;
        }
    }
    Ok(())
}

pub fn run_google_music(
    config: &Config,
    filename: Option<&str>,
    do_add: bool,
) -> Result<(), Error> {
    if let Some(fname) = filename {
        if Path::new(fname).exists() {
            if do_add {
                let flist: Vec<_> = BufReader::new(File::open(fname)?)
                    .lines()
                    .into_iter()
                    .map(|l| {
                        let line = l?;
                        let p = Path::new(&line);
                        Ok(p.to_path_buf())
                    })
                    .collect();
                let flist: Vec<_> = map_result(flist)?;
                return upload_list_of_mp3s(&flist).map_err(|e| err_msg(format!("{:?}", e)));
            }
        }
    }

    let results: Vec<_> = get_uploaded_mp3()
        .map_err(|e| err_msg(format!("{:?}", e)))?
        .into_par_iter()
        .map(|line| {
            let m: GoogleMusicMetadata = serde_json::from_str(&line)?;
            Ok(m)
        })
        .collect();

    let metadata: Vec<_> = map_result(results)?;

    let title_map: HashMap<_, _> = metadata.iter().map(|m| (m.title.clone(), m)).collect();

    let wdir = WalkDir::new(&config.google_music_directory);
    let entries: Vec<_> = wdir.into_iter().filter_map(Result::ok).collect();

    let all_files: Vec<_> = entries
        .into_par_iter()
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect();

    let has_tag: HashMap<_, _> = all_files
        .par_iter()
        .filter_map(|path| {
            if let Ok(tag) = Tag::read_from_path(&path) {
                Some((path.clone(), tag))
            } else {
                None
            }
        })
        .collect();

    let no_tag: Vec<_> = all_files
        .par_iter()
        .filter_map(|path| {
            if has_tag.contains_key(path) {
                None
            } else {
                Some(path)
            }
        })
        .collect();

    let not_in_metadata: Vec<_> = has_tag
        .par_iter()
        .filter_map(|(p, t)| {
            if let Some(title) = t.title() {
                if !title_map.contains_key(title) {
                    for title_part in title.split("-") {
                        if title_map.contains_key(title_part.trim()) {
                            return None;
                        }
                    }
                    if title_map.contains_key(&title.replace("--", "-")) {
                        return None;
                    }
                    for key in title_map.keys() {
                        if title.contains(key) {
                            println!("exising key :{}: , :{}:", key, title);
                        }
                    }
                    println!("{} {:?}", title, p);
                    Some(p.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    println!(
        "{} {} {} {}",
        all_files.len(),
        has_tag.len(),
        not_in_metadata.len(),
        no_tag.len(),
    );

    if let Some(fname) = filename {
        let mut f = File::create(fname)?;
        for p in not_in_metadata {
            if let Some(s) = p.to_str() {
                writeln!(f, "{}", s)?;
            }
        }
    } else if do_add {
        upload_list_of_mp3s(&not_in_metadata).map_err(|e| err_msg(format!("{:?}", e)))?;
    }

    Ok(())
}
