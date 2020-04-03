use anyhow::{format_err, Error};
use cpython::{
    exc, FromPyObject, ObjectProtocol, PyDict, PyErr, PyList, PyObject, PyResult, PyString,
    PyTuple, Python, PythonObject, ToPyObject,
};
use futures::future::try_join_all;
use id3::Tag;
use log::debug;
use postgres_query::FromSqlRow;
use serde::Deserialize;
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::File,
    io::{BufRead, BufReader, Write},
    iter::Iterator,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::task::spawn_blocking;
use walkdir::WalkDir;

use crate::{config::Config, pgpool::PgPool, stdout_channel::StdoutChannel};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MusicKey {
    pub artist: String,
    pub album: String,
    pub title: String,
    pub track_number: Option<i32>,
}

#[derive(Deserialize, Debug, Clone, FromSqlRow)]
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
    pub async fn insert_into_db(&self, pool: &PgPool) -> Result<(), Error> {
        let query = postgres_query::query!(
            r#"
            INSERT INTO google_music_metadata (
                id, title, album, artist, track_size, album_artist, track_number, disc_number,
                total_disc_count, filename
            )
            VALUES ($id, $title, $album, $artist, $track_size, $album_artist, $track_number, $disc_number,
                $total_disc_count, $filename)
        "#,
            id = self.id,
            title = self.title,
            album = self.album,
            artist = self.artist,
            track_size = self.track_size,
            album_artist = self.album_artist,
            track_number = self.track_number,
            disc_number = self.disc_number,
            total_disc_count = self.total_disc_count,
            filename = self.filename
        );
        pool.get()
            .await?
            .execute(query.sql(), &query.parameters())
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    pub async fn update_db(&self, pool: &PgPool) -> Result<(), Error> {
        let query = postgres_query::query!(
            r#"
                UPDATE google_music_metadata
                SET track_size=$track_size,album_artist=$album_artist,track_number=$track_number,
                    disc_number=$disc_number,total_disc_count=$total_disc_count,filename=$filename
                WHERE id=$id AND title=$title AND album=$album AND artist=$artist
            "#,
            id = self.id,
            title = self.title,
            album = self.album,
            artist = self.artist,
            track_size = self.track_size,
            album_artist = self.album_artist,
            track_number = self.track_number,
            disc_number = self.disc_number,
            total_disc_count = self.total_disc_count,
            filename = self.filename
        );
        pool.get()
            .await?
            .execute(query.sql(), &query.parameters())
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    pub async fn by_id(id: &str, pool: &PgPool) -> Result<Option<Self>, Error> {
        let query = r#"
            SELECT
                id, title, album, artist, track_size, album_artist, track_number, disc_number,
                total_disc_count, filename
            FROM google_music_metadata
            WHERE id=$1
        "#;
        if let Some(row) = pool.get().await?.query(query, &[&id]).await?.get(0) {
            let g = Self::from_row(row)?;
            Ok(Some(g))
        } else {
            Ok(None)
        }
    }

    pub async fn by_key(key: &MusicKey, pool: &PgPool) -> Result<Vec<Self>, Error> {
        let query = r#"
            SELECT
                id, title, album, artist, track_size, album_artist, track_number, disc_number,
                total_disc_count, filename
            FROM google_music_metadata
            WHERE artist=$1 AND album=$2 AND title=$3
        "#;
        pool.get()
            .await?
            .query(query, &[&key.artist, &key.album, &key.title])
            .await?
            .iter()
            .map(|row| {
                let g = Self::from_row(row)?;
                Ok(g)
            })
            .collect()
    }

    pub async fn by_title(title: &str, pool: &PgPool) -> Result<Vec<Self>, Error> {
        let query = r#"
            SELECT
                id, title, album, artist, track_size, album_artist, track_number, disc_number,
                total_disc_count, filename
            FROM google_music_metadata
            WHERE title=$1
        "#;
        pool.get()
            .await?
            .query(query, &[&title])
            .await?
            .iter()
            .map(|row| {
                let g = Self::from_row(row)?;
                Ok(g)
            })
            .collect()
    }

    pub fn from_pydict(py: Python, dict: &PyDict) -> PyResult<Self> {
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

    pub fn get_uploaded_mp3(config: &Config) -> Result<Vec<Self>, Error> {
        _get_uploaded_mp3(config).map_err(|e| format_err!("{:?}", e))
    }
}

fn exception(py: Python, msg: &str) -> PyErr {
    PyErr::new::<exc::Exception, _>(py, msg)
}

fn _get_uploaded_mp3(config: &Config) -> PyResult<Vec<GoogleMusicMetadata>> {
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
        let result = GoogleMusicMetadata::from_pydict(py, &dict)?;
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
        let s = p.to_string_lossy();
        debug!("upload {}", s);
        let fname = PyString::new(py, &s);
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
    Ok(results)
}

pub async fn run_google_music(
    config: &Config,
    metadata: Vec<GoogleMusicMetadata>,
    filename: Option<&str>,
    do_add: bool,
    pool: &PgPool,
    stdout: &StdoutChannel,
) -> Result<(), Error> {
    if let Some(fname) = filename {
        if Path::new(fname).exists() && do_add {
            let flist: Result<Vec<_>, Error> = BufReader::new(File::open(fname)?)
                .lines()
                .map(|l| {
                    let line = l?;
                    let p: PathBuf = line.into();
                    Ok(p)
                })
                .collect();
            let flist = flist?;
            let config = config.clone();
            let stdout = stdout.clone();
            return spawn_blocking(move || {
                let ids =
                    upload_list_of_mp3s(&config, &flist).map_err(|e| format_err!("{:?}", e))?;
                for id in ids {
                    if let Some(id) = id {
                        stdout.send(format!("upload {}", id))?;
                    }
                }
                Ok(())
            })
            .await?;
        }
    }

    let futures: Vec<_> = metadata
        .into_iter()
        .map(|mut m| {
            let pool = pool.clone();
            async move {
                if let Some(m_) = GoogleMusicMetadata::by_id(&m.id, &pool).await? {
                    m.filename = m_.filename;
                } else {
                    m.insert_into_db(&pool).await?;
                }
                Ok(m)
            }
        })
        .collect();
    let metadata: Result<Vec<_>, Error> = try_join_all(futures).await;
    let metadata = metadata?;

    let filename_map: HashMap<String, _> = metadata
        .iter()
        .filter_map(|m| m.filename.as_ref().map(|f| (f.to_string(), m)))
        .collect();

    debug!("filename_map {}", filename_map.len());

    let title_map: HashMap<_, _> = metadata.iter().map(|m| (m.title.to_string(), m)).collect();
    let title_map = Arc::new(title_map);

    let futures: Vec<_> = title_map
        .keys()
        .map(|t| {
            let t = t.to_string();
            let pool = pool.clone();
            async move {
                let items = GoogleMusicMetadata::by_title(&t, &pool).await?;
                Ok((t, items))
            }
        })
        .collect();
    let results: Result<Vec<_>, Error> = try_join_all(futures).await;
    let title_db_map: HashMap<String, _> = results?.into_iter().collect();
    let title_db_map = Arc::new(title_db_map);

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
    let key_map = Arc::new(key_map);

    let wdir = WalkDir::new(&config.google_music_directory);
    let entries: Vec<_> = wdir.into_iter().filter_map(Result::ok).collect();

    let all_files: Vec<_> = entries
        .into_iter()
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let p = entry.into_path();
            let s = p.to_string_lossy();
            if filename_map.contains_key(s.as_ref()) {
                return None;
            }
            Some(p)
        })
        .collect();

    let has_tag: HashMap<_, _> = all_files
        .iter()
        .filter_map(|path| {
            if let Ok(tag) = Tag::read_from_path(&path) {
                Some((path.clone(), tag))
            } else {
                None
            }
        })
        .collect();

    let has_tag = Arc::new(has_tag);

    let futures: Vec<_> = all_files
        .iter()
        .map(|path| {
            let has_tag = has_tag.clone();
            let title_map = title_map.clone();
            let title_db_map = title_db_map.clone();
            async move {
                if has_tag.contains_key(path) {
                    return Ok(None);
                }
                if let Some(title) = path.file_name().map(OsStr::to_string_lossy) {
                    if let Some(items) = title_db_map.get(title.as_ref()) {
                        if items.len() == 1 {
                            if let Some(m) = title_map.get(title.as_ref()) {
                                if m.filename.is_none() {
                                    let mut m = (*(*m)).clone();
                                    m.filename.replace(path.to_string_lossy().to_string());
                                    m.update_db(&pool).await?;
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
                Ok(Some(path))
            }
        })
        .collect();
    let results: Result<Vec<_>, Error> = try_join_all(futures).await;
    let no_tag: Vec<_> = results?.into_iter().filter_map(|x| x).collect();

    let futures: Vec<_> = has_tag
        .iter()
        .map(|(p, t)| {
            let key_map = key_map.clone();
            async move {
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
                                if m.filename.is_none() {
                                    let mut m = (*(*m)).clone();
                                    m.filename.replace(p.to_string_lossy().to_string());
                                    m.update_db(&pool).await?;
                                }
                                return Ok(Some((k, p)));
                            }
                        }
                    }
                }
                Ok(None)
            }
        })
        .collect();
    let results: Result<Vec<_>, Error> = try_join_all(futures).await;
    let in_music_key: HashMap<_, _> = results?.into_iter().filter_map(|x| x).collect();

    let futures: Vec<_> = has_tag
        .iter()
        .map(|(p, t)| {
            let title_map = title_map.clone();
            let title_db_map = title_db_map.clone();
            async move {
                if let Some(title) = t.title() {
                    if let Some(items) = title_db_map.get(title) {
                        if items.len() == 1 {
                            if let Some(m) = title_map.get(title) {
                                if m.filename.is_none() {
                                    let mut m = (*(*m)).clone();
                                    m.filename.replace(p.to_string_lossy().to_string());
                                    m.update_db(&pool).await?;
                                }
                            }
                        } else {
                            for item in items {
                                if item.filename.is_none() {
                                    debug!("title no filename {:?} {} {}", p, title, item.id);
                                }
                            }
                        }
                    } else {
                        for title_part in title.split('-') {
                            if title_db_map.contains_key(title_part.trim()) {
                                return Ok(None);
                            }
                        }
                        if title_db_map.contains_key(&title.replace("--", "-")) {
                            return Ok(None);
                        }
                        for key in title_db_map.keys() {
                            if title.contains(key) {
                                debug!("exising key :{}: , :{}:", key, title);
                            }
                        }
                        debug!("no title {} {:?}", title, p);
                        return Ok(Some(p.to_owned()));
                    }
                }
                Ok(None)
            }
        })
        .collect();
    let results: Result<Vec<_>, Error> = try_join_all(futures).await;
    let not_in_metadata: Vec<_> = results?.into_iter().filter_map(|x| x).collect();

    stdout.send(format!(
        "all:{} tag:{} in_music_key:{} not_in_metadata:{} no_tag:{}",
        all_files.len(),
        has_tag.len(),
        in_music_key.len(),
        not_in_metadata.len(),
        no_tag.len(),
    ))?;

    if let Some(fname) = filename {
        let mut f = File::create(fname)?;
        for p in not_in_metadata {
            writeln!(f, "{}", p.to_string_lossy())?;
        }
    } else if do_add {
        upload_list_of_mp3s(config, &not_in_metadata).map_err(|e| format_err!("{:?}", e))?;
    }

    Ok(())
}
