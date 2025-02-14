#[cfg(feature = "cli")]
use clap::ValueEnum;
use std::ffi::OsStr;
use std::fmt;

use crate::{FileFormatOptions, SkywayError};

/// Enum that represents the different OSM file formats skyway supports.
#[cfg(feature = "cli")]
#[derive(Clone, Debug, ValueEnum)]
#[non_exhaustive]
pub enum OsmFormat {
    #[cfg(feature = "json")]
    #[value(name = "json")]
    Json,
    #[cfg(feature = "o5m")]
    #[value(name = "o5m")]
    O5m,
    #[cfg(feature = "opl")]
    #[value(name = "opl")]
    Opl,
    #[cfg(feature = "json")]
    #[value(name = "overpass")]
    Overpass,
    #[cfg(feature = "xml")]
    #[value(name = "xml", alias = "osm")]
    Xml,
    #[cfg(feature = "pbf")]
    #[value(name = "pbf")]
    Pbf,
}

#[cfg(not(feature = "cli"))]
#[derive(Clone, Debug)]
pub enum OsmFormat {
    #[cfg(feature = "json")]
    Json,
    #[cfg(feature = "o5m")]
    O5m,
    #[cfg(feature = "opl")]
    Opl,
    #[cfg(feature = "json")]
    Overpass,
    #[cfg(feature = "xml")]
    Xml,
    #[cfg(feature = "pbf")]
    Pbf,
}

#[cfg(feature = "cli")]
impl FileFormatOptions for OsmFormat {
    fn format_error(ext: &str) -> SkywayError {
        SkywayError::UnknownFormat(ext.to_string())
    }
}

impl OsmFormat {
    /// Return if the `OsmFormat` can be decoded by the lib.
    pub const fn can_read(&self) -> bool {
        match self {
            OsmFormat::Json => true,
            OsmFormat::O5m => false,
            OsmFormat::Opl => true,
            OsmFormat::Overpass => true,
            OsmFormat::Xml => true,
            OsmFormat::Pbf => true,
        }
    }

    /// Return if the `OsmFormat` can be encoded by the lib.
    pub const fn can_write(&self) -> bool {
        match self {
            OsmFormat::Json => true,
            OsmFormat::O5m => true,
            OsmFormat::Opl => true,
            OsmFormat::Overpass => true,
            OsmFormat::Xml => true,
            OsmFormat::Pbf => false,
        }
    }

    /// Return the `OsmFormat`s which are enabled for reading.
    #[inline]
    #[must_use]
    pub fn reading_enabled(&self) -> bool {
        match self {
            OsmFormat::Json => cfg!(feature = "json"),
            OsmFormat::O5m => false,
            OsmFormat::Opl => cfg!(feature = "opl"),
            OsmFormat::Overpass => cfg!(feature = "json"),
            OsmFormat::Xml => cfg!(feature = "xml"),
            OsmFormat::Pbf => cfg!(feature = "pbf"),
        }
    }

    /// Return the `OsmFormat`s which are enabled for writing.
    #[inline]
    #[must_use]
    pub fn writing_enabled(&self) -> bool {
        match self {
            OsmFormat::Json => cfg!(feature = "json"),
            OsmFormat::O5m => cfg!(feature = "o5m"),
            OsmFormat::Opl => cfg!(feature = "opl"),
            OsmFormat::Overpass => cfg!(feature = "json"),
            OsmFormat::Xml => cfg!(feature = "xml"),
            OsmFormat::Pbf => false,
        }
    }

    /// Validates format capabilities (can be evaluated at compile time)
    #[inline]
    pub fn validate_capabilities(input: &OsmFormat, output: &OsmFormat) -> Result<(), SkywayError> {
        if !input.can_read() {
            return Err(SkywayError::UnsupportedRead(format!(
                "format {:?} does not support reading",
                input
            )));
        }

        if !output.can_write() {
            return Err(SkywayError::UnsupportedWrite(format!(
                "format {:?} does not support writing",
                output
            )));
        }

        Ok(())
    }

    /// Validates feature flags are enabled
    #[inline]
    pub fn validate_enabled(input: &OsmFormat, output: &OsmFormat) -> Result<(), SkywayError> {
        if !input.reading_enabled() {
            return Err(SkywayError::UnsupportedRead(format!(
                "reading support not enabled for format {:?}",
                input
            )));
        }

        if !output.writing_enabled() {
            return Err(SkywayError::UnsupportedWrite(format!(
                "writing support not enabled for format {:?}",
                output
            )));
        }

        Ok(())
    }

    /// Full validation of both capabilities and enabled features
    pub fn validate_conversion(input: &OsmFormat, output: &OsmFormat) -> Result<(), SkywayError> {
        Self::validate_capabilities(input, output)?;
        Self::validate_enabled(input, output)?;
        Ok(())
    }

    /// Return the format from a file extension
    #[inline]
    pub fn from_extension<S>(ext: S) -> Option<Self>
    where
        S: AsRef<OsStr>,
    {
        fn inner(ext: &OsStr) -> Option<OsmFormat> {
            let ext = ext.to_str()?.to_ascii_lowercase();

            Some(match ext.as_str() {
                "json" => OsmFormat::Json,
                "o5m" => OsmFormat::O5m,
                "opl" => OsmFormat::Opl,
                "osm" | "xml" => OsmFormat::Xml,
                "pbf" => OsmFormat::Pbf,
                _ => return None,
            })
        }

        inner(ext.as_ref())
    }
}

impl fmt::Display for OsmFormat {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OsmFormat::Json => write!(f, "json")?,
            OsmFormat::O5m => write!(f, "o5m")?,
            OsmFormat::Opl => write!(f, "opl")?,
            OsmFormat::Overpass => write!(f, "overpass")?,
            OsmFormat::Xml => write!(f, "xml")?,
            OsmFormat::Pbf => write!(f, "pbf")?,
        }

        Ok(())
    }
}
