use serde::Deserialize;
use cpython::{
    exc, FromPyObject, ObjectProtocol, PyDict, PyErr, PyList, PyObject, PyResult, PyString,
    PyTuple, Python, PythonObject, ToPyObject,
};
use failure::{format_err, Error};
use id3::Tag;
use log::debug;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::collections::HashMap;
use std::fs::File;
use std::io::{stdout, BufRead, BufReader, Write};
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::Config;
use crate::pgpool::PgPool;
use crate::row_index_trait::RowIndexTrait;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MusicKey {
    pub artist: String,
    pub album: String,
    pub title: String,
    pub track_number: Option<i32>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GoogleMusicMetadata {
    pub id: String,
    pub title: String,
    pub album: String,
    pub artist: String,
    pub track_size: i32,
    pub album_artist: Option<String>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub total_disc_count: Option<i32>,
    pub filename: Option<String>,
}

macro_rules! get_pydict_item_option {
    ($py:ident, $dict:ident, $id:ident, $T:ty) => {
        $dict
            .get_item($py, &stringify!($id))
            .as_ref()
            .map(|v| <$T>::extract($py, v))
            .transpose()
    };
}

macro_rules! get_pydict_item {
    ($py:ident, $dict:ident, $id:ident, $T:ty) => {
        get_pydict_item_option!($py, $dict, $id, $T)
            .and_then(|x| x.ok_or_else(|| exception($py, &format!("No {}", stringify!($id)))))
    };
}

impl GoogleMusicMetadata {
    pub fn insert_into_db(&self, pool: &PgPool) -> Result<(), Error> {
        let query = r#"
            INSERT INTO google_music_metadata (
                id, title, album, artist, track_size, album_artist, track_number, disc_number,
                total_disc_count, filename
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#;
        pool.get()?.execute(
            query,
            &[
                &self.id,
                &self.title,
                &self.album,
                &self.artist,
                &self.track_size,
                &self.album_artist,
                &self.track_number,
                &self.disc_number,
                &self.total_disc_count,
                &self.filename,
            ],
        )?;
        Ok(())
    }

    pub fn update_db(&self, pool: &PgPool) -> Result<(), Error> {
        let query = r#"
            UPDATE google_music_metadata
            SET track_size=$5,album_artist=$6,track_number=$7,disc_number=$8,total_disc_count=$9,
                filename=$10
            WHERE id=$1 AND title=$2 AND album=$3 AND artist=$4
        "#;
        pool.get()?.execute(
            query,
            &[
                &self.id,
                &self.title,
                &self.album,
                &self.artist,
                &self.track_size,
                &self.album_artist,
                &self.track_number,
                &self.disc_number,
                &self.total_disc_count,
                &self.filename,
            ],
        )?;
        Ok(())
    }

    pub fn by_id(id: &str, pool: &PgPool) -> Result<Option<GoogleMusicMetadata>, Error> {
        let query = r#"
            SELECT
                id, title, album, artist, track_size, album_artist, track_number, disc_number,
                total_disc_count, filename
            FROM google_music_metadata
            WHERE id=$1
        "#;
        if let Some(row) = pool.get()?.query(query, &[&id])?.iter().nth(0) {
            let id: String = row.get_idx(0)?;
            let title: String = row.get_idx(1)?;
            let album: String = row.get_idx(2)?;
            let artist: String = row.get_idx(3)?;
            let track_size: i32 = row.get_idx(4)?;
            let album_artist: Option<String> = row.get_idx(5)?;
            let track_number: Option<i32> = row.get_idx(6)?;
            let disc_number: Option<i32> = row.get_idx(7)?;
            let total_disc_count: Option<i32> = row.get_idx(8)?;
            let filename: Option<String> = row.get_idx(9)?;
            let g = GoogleMusicMetadata {
                id,
                title,
                album,
                artist,
                track_size,
                album_artist,
                track_number,
                disc_number,
                total_disc_count,
                filename,
            };
            Ok(Some(g))
        } else {
            Ok(None)
        }
    }

    pub fn by_key(key: &MusicKey, pool: &PgPool) -> Result<Vec<GoogleMusicMetadata>, Error> {
        let query = r#"
            SELECT
                id, title, album, artist, track_size, album_artist, track_number, disc_number,
                total_disc_count, filename
            FROM google_music_metadata
            WHERE artist=$1 AND album=$2 AND title=$3
        "#;
        pool.get()?
            .query(query, &[&key.artist, &key.album, &key.title])?
            .iter()
            .map(|row| {
                let id: String = row.get_idx(0)?;
                let title: String = row.get_idx(1)?;
                let album: String = row.get_idx(2)?;
                let artist: String = row.get_idx(3)?;
                let track_size: i32 = row.get_idx(4)?;
                let album_artist: Option<String> = row.get_idx(5)?;
                let track_number: Option<i32> = row.get_idx(6)?;
                let disc_number: Option<i32> = row.get_idx(7)?;
                let total_disc_count: Option<i32> = row.get_idx(8)?;
                let filename: Option<String> = row.get_idx(9)?;
                let g = GoogleMusicMetadata {
                    id,
                    title,
                    album,
                    artist,
                    track_size,
                    album_artist,
                    track_number,
                    disc_number,
                    total_disc_count,
                    filename,
                };
                Ok(g)
            })
            .collect()
    }

    pub fn by_title(title: &str, pool: &PgPool) -> Result<Vec<GoogleMusicMetadata>, Error> {
        let query = r#"
            SELECT
                id, title, album, artist, track_size, album_artist, track_number, disc_number,
                total_disc_count, filename
            FROM google_music_metadata
            WHERE title=$1
        "#;
        pool.get()?
            .query(query, &[&title])?
            .iter()
            .map(|row| {
                let id: String = row.get_idx(0)?;
                let title: String = row.get_idx(1)?;
                let album: String = row.get_idx(2)?;
                let artist: String = row.get_idx(3)?;
                let track_size: i32 = row.get_idx(4)?;
                let album_artist: Option<String> = row.get_idx(5)?;
                let track_number: Option<i32> = row.get_idx(6)?;
                let disc_number: Option<i32> = row.get_idx(7)?;
                let total_disc_count: Option<i32> = row.get_idx(8)?;
                let filename: Option<String> = row.get_idx(9)?;
                let g = GoogleMusicMetadata {
                    id,
                    title,
                    album,
                    artist,
                    track_size,
                    album_artist,
                    track_number,
                    disc_number,
                    total_disc_count,
                    filename,
                };
                Ok(g)
            })
            .collect()
    }

    pub fn from_pydict(py: Python, dict: PyDict) -> PyResult<Self> {
        let id = get_pydict_item!(py, dict, id, String)?;
        let title = get_pydict_item!(py, dict, title, String)?;
        let album = get_pydict_item!(py, dict, album, String)?;
        let artist = get_pydict_item!(py, dict, artist, String)?;
        let track_size = get_pydict_item!(py, dict, track_size, i32)?;
        let album_artist = get_pydict_item_option!(py, dict, album_artist, String)?;
        let track_number = get_pydict_item_option!(py, dict, track_number, i32)?;
        let disc_number = get_pydict_item_option!(py, dict, disc_number, i32)?;
        let total_disc_count = get_pydict_item_option!(py, dict, total_disc_count, i32)?;
        let filename = get_pydict_item_option!(py, dict, filename, String)?;

        let gm = Self {
            id,
            title,
            album,
            artist,
            track_size,
            album_artist,
            track_number,
            disc_number,
            total_disc_count,
            filename,
        };

        Ok(gm)
    }
}

fn exception(py: Python, msg: &str) -> PyErr {
    PyErr::new::<exc::Exception, _>(py, msg)
}

pub fn get_uploaded_mp3(config: &Config) -> PyResult<Vec<GoogleMusicMetadata>> {
    let gil = Python::acquire_gil();
    let py = gil.python();
    let google_music = py.import("google_music")?;
    let mm: PyObject = google_music.call(
        py,
        "MusicManager",
        PyTuple::new(py, &[config.user.to_py_object(py).into_object()]),
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
        let result = GoogleMusicMetadata::from_pydict(py, dict)?;
        results.push(result);
    }
    Ok(results)
}

pub fn upload_list_of_mp3s(config: &Config, filelist: &[PathBuf]) -> PyResult<Vec<Option<String>>> {
    let gil = Python::acquire_gil();
    let py = gil.python();
    let google_music = py.import("google_music")?;
    let mm: PyObject = google_music.call(
        py,
        "MusicManager",
        PyTuple::new(py, &[config.user.to_py_object(py).into_object()]),
        None,
    )?;
    let mut results = Vec::new();
    for p in filelist {
        if let Some(s) = p.to_str() {
            debug!("upload {}", s);
            let fname = PyString::new(py, s);
            let result: PyObject =
                mm.call_method(py, "upload", PyTuple::new(py, &[fname.into_object()]), None)?;
            let result = PyDict::extract(py, &result)?;
            let id = match result.get_item(py, "song_id") {
                Some(s) => {
                    let id = PyString::extract(py, &s)?.to_string(py)?.to_string();
                    let output = format!("{} {}", s, id);
                    Some(output)
                }
                None => None,
            };
            results.push(id);
        }
    }
    Ok(results)
}

pub fn run_google_music(
    config: &Config,
    filename: Option<&str>,
    do_add: bool,
    pool: &PgPool,
) -> Result<(), Error> {
    if let Some(fname) = filename {
        if Path::new(fname).exists() && do_add {
            let flist: Result<Vec<_>, Error> = BufReader::new(File::open(fname)?)
                .lines()
                .map(|l| {
                    let line = l?;
                    let p = Path::new(&line);
                    Ok(p.to_path_buf())
                })
                .collect();
            let ids = upload_list_of_mp3s(config, &flist?).map_err(|e| format_err!("{:?}", e))?;
            for id in ids {
                if let Some(id) = id {
                    writeln!(stdout().lock(), "upload {}", id)?;
                }
            }
            return Ok(());
        }
    }

    let results: Result<Vec<_>, Error> = get_uploaded_mp3(config)
        .map_err(|e| format_err!("{:?}", e))?
        .into_par_iter()
        .map(|mut m| {
            if let Some(m_) = GoogleMusicMetadata::by_id(&m.id, &pool)? {
                m.filename = m_.filename;
            } else {
                m.insert_into_db(&pool)?;
            }
            Ok(m)
        })
        .collect();

    let metadata = results?;

    let filename_map: HashMap<String, _> = metadata
        .par_iter()
        .filter_map(|m| m.filename.as_ref().map(|f| (f.to_string(), m)))
        .collect();

    debug!("filename_map {}", filename_map.len());

    let title_map: HashMap<_, _> = metadata.iter().map(|m| (m.title.to_string(), m)).collect();

    let results: Result<HashMap<_, _>, Error> = title_map
        .keys()
        .map(|t| {
            let items = GoogleMusicMetadata::by_title(t, &pool)?;
            Ok((t.to_string(), items))
        })
        .collect();

    let title_db_map = results?;

    let key_map: HashMap<_, _> = metadata
        .iter()
        .map(|m| {
            let k = MusicKey {
                artist: m.artist.to_string(),
                album: m.album.to_string(),
                title: m.title.to_string(),
                track_number: m.track_number,
            };
            (k, m)
        })
        .collect();

    let wdir = WalkDir::new(&config.google_music_directory);
    let entries: Vec<_> = wdir.into_iter().filter_map(Result::ok).collect();

    let all_files: Vec<_> = entries
        .into_par_iter()
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let p = entry.into_path();
            if let Some(s) = p.to_str() {
                if filename_map.contains_key(s) {
                    return None;
                }
            }
            Some(p)
        })
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
                if let Some(title) = path.file_name().and_then(|s| s.to_str()) {
                    if let Some(items) = title_db_map.get(title) {
                        if items.len() == 1 {
                            if let Some(m) = title_map.get(title) {
                                if m.filename.is_none() {
                                    if let Some(s) = path.to_str() {
                                        let mut m = (*(*m)).clone();
                                        m.filename.replace(s.to_string());
                                        m.update_db(&pool).unwrap();
                                    }
                                }
                            }
                        } else {
                            for item in items {
                                if item.filename.is_none() {
                                    debug!("no tag no filename {:?} {} {}", path, title, item.id);
                                }
                            }
                        }
                    }
                }
                Some(path)
            }
        })
        .collect();

    let in_music_key: HashMap<_, _> = has_tag
        .par_iter()
        .filter_map(|(p, t)| {
            if let Some(title) = t.title() {
                if let Some(artist) = t.artist() {
                    if let Some(album) = t.album() {
                        let k = MusicKey {
                            artist: artist.to_string(),
                            album: album.to_string(),
                            title: title.to_string(),
                            track_number: t.track().map(|x| x as i32),
                        };
                        if let Some(m) = key_map.get(&k) {
                            if let Some(s) = p.to_str() {
                                if m.filename.is_none() {
                                    let mut m = (*(*m)).clone();
                                    m.filename.replace(s.to_string());
                                    m.update_db(&pool).unwrap();
                                }
                            }
                            return Some((k, p));
                        }
                    }
                }
            }
            None
        })
        .collect();

    let not_in_metadata: Vec<_> = has_tag
        .par_iter()
        .filter_map(|(p, t)| {
            if let Some(title) = t.title() {
                if let Some(items) = title_db_map.get(title) {
                    if items.len() == 1 {
                        if let Some(m) = title_map.get(title) {
                            if m.filename.is_none() {
                                if let Some(s) = p.to_str() {
                                    let mut m = (*(*m)).clone();
                                    m.filename.replace(s.to_string());
                                    m.update_db(&pool).unwrap();
                                }
                            }
                        }
                    } else {
                        for item in items {
                            if item.filename.is_none() {
                                debug!("title no filename {:?} {} {}", p, title, item.id);
                            }
                        }
                    }
                    None
                } else {
                    for title_part in title.split('-') {
                        if title_db_map.contains_key(title_part.trim()) {
                            return None;
                        }
                    }
                    if title_db_map.contains_key(&title.replace("--", "-")) {
                        return None;
                    }
                    for key in title_db_map.keys() {
                        if title.contains(key) {
                            debug!("exising key :{}: , :{}:", key, title);
                        }
                    }
                    debug!("no title {} {:?}", title, p);
                    Some(p.to_owned())
                }
            } else {
                None
            }
        })
        .collect();

    writeln!(
        stdout().lock(),
        "all:{} tag:{} in_music_key:{} not_in_metadata:{} no_tag:{}",
        all_files.len(),
        has_tag.len(),
        in_music_key.len(),
        not_in_metadata.len(),
        no_tag.len(),
    )?;

    if let Some(fname) = filename {
        let mut f = File::create(fname)?;
        for p in not_in_metadata {
            if let Some(s) = p.to_str() {
                writeln!(f, "{}", s)?;
            }
        }
    } else if do_add {
        upload_list_of_mp3s(config, &not_in_metadata).map_err(|e| format_err!("{:?}", e))?;
    }

    Ok(())
}
