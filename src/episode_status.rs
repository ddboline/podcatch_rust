use anyhow::{format_err, Error};
use bytes::BytesMut;
use postgres::types::{FromSql, IsNull, ToSql, Type};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EpisodeStatus {
    Ready,
    Downloaded,
    Error,
    Skipped,
}

impl fmt::Display for EpisodeStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::Ready => "Ready",
            Self::Downloaded => "Downloaded",
            Self::Error => "Error",
            Self::Skipped => "Skipped",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for EpisodeStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Ready" => Ok(Self::Ready),
            "Downloaded" => Ok(Self::Downloaded),
            "Error" => Ok(Self::Error),
            "Skipped" => Ok(Self::Skipped),
            _ => Err(format_err!("Invalid string {}", s)),
        }
    }
}

impl Default for EpisodeStatus {
    fn default() -> Self {
        Self::Ready
    }
}

impl<'a> FromSql<'a> for EpisodeStatus {
    fn from_sql(
        ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = String::from_sql(ty, raw)?.parse()?;
        Ok(s)
    }

    fn accepts(ty: &Type) -> bool {
        <String as FromSql>::accepts(ty)
    }
}

impl ToSql for EpisodeStatus {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>>
    where
        Self: Sized,
    {
        self.to_string().to_sql(ty, out)
    }

    fn accepts(ty: &Type) -> bool
    where
        Self: Sized,
    {
        <String as ToSql>::accepts(ty)
    }

    fn to_sql_checked(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        self.to_string().to_sql_checked(ty, out)
    }
}
