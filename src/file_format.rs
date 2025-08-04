#[cfg(feature = "cli")]
use clap::ValueEnum;
use std::ffi::OsStr;
use std::fmt;
use std::path::PathBuf;

use crate::SkywayError;

/// Enum that represents the different OSM file formats skyway supports.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
#[non_exhaustive]
pub enum OsmFormat {
    #[cfg_attr(feature = "cli", value(name = "geojson"))]
    GeoJson,
    #[cfg_attr(feature = "cli", value(name = "json"))]
    Json,
    #[cfg_attr(feature = "cli", value(name = "o5m"))]
    O5m,
    #[cfg_attr(feature = "cli", value(name = "opl"))]
    Opl,
    #[cfg_attr(feature = "cli", value(name = "overpass"))]
    Overpass,
    #[cfg_attr(feature = "cli", value(name = "xml", alias = "osm"))]
    Xml,
    #[cfg_attr(feature = "cli", value(name = "pbf"))]
    Pbf,
}

impl OsmFormat {
    fn format_error(ext: &str) -> SkywayError {
        SkywayError::UnknownFormat(ext.to_string())
    }

    pub fn parse(
        cli_format: Option<Self>,
        file_path: &Option<PathBuf>,
    ) -> Result<Self, SkywayError> {
        if let Some(format) = cli_format {
            Ok(format)
        } else {
            let path = file_path.as_ref().ok_or(Self::format_error(
                "no file path given or format specified.",
            ))?;

            let extension = path.extension().ok_or(Self::format_error(
                format!(
                    "unable to extract extension from path \"{}\"",
                    path.display()
                )
                .as_str(),
            ))?;

            let ext_str = extension.to_str().ok_or(Self::format_error(
                format!(
                    "extension found but could not be converted to a string in path \"{}\"",
                    path.display()
                )
                .as_str(),
            ))?;

            Self::from_extension(ext_str).ok_or(Self::format_error(
                format!("File extension not recognized: {}", ext_str).as_str(),
            ))
        }
    }

    /// Return if the `OsmFormat` can be decoded by the lib.
    pub const fn can_read(&self) -> bool {
        match self {
            #[cfg(feature = "json")]
            OsmFormat::Json => true,
            #[cfg(feature = "json")]
            OsmFormat::Overpass => true,
            #[cfg(feature = "opl")]
            OsmFormat::Opl => true,
            #[cfg(feature = "xml")]
            OsmFormat::Xml => true,
            #[cfg(feature = "pbf")]
            OsmFormat::Pbf => true,
            _ => false,
        }
    }

    /// Return if the `OsmFormat` can be encoded by the lib.
    pub const fn can_write(&self) -> bool {
        match self {
            #[cfg(feature = "geojson")]
            OsmFormat::GeoJson => true,
            #[cfg(feature = "json")]
            OsmFormat::Json => true,
            #[cfg(feature = "json")]
            OsmFormat::Overpass => true,
            #[cfg(feature = "o5m")]
            OsmFormat::O5m => true,
            #[cfg(feature = "opl")]
            OsmFormat::Opl => true,
            #[cfg(feature = "xml")]
            OsmFormat::Xml => true,
            _ => false,
        }
    }

    /// Return the `OsmFormat`s which are enabled for reading.
    #[inline]
    #[must_use]
    pub fn reading_enabled(&self) -> bool {
        match self {
            #[cfg(feature = "json")]
            OsmFormat::Json => true,
            #[cfg(feature = "json")]
            OsmFormat::Overpass => true,
            #[cfg(feature = "opl")]
            OsmFormat::Opl => true,
            #[cfg(feature = "xml")]
            OsmFormat::Xml => true,
            #[cfg(feature = "pbf")]
            OsmFormat::Pbf => true,
            _ => false,
        }
    }

    /// Return the `OsmFormat`s which are enabled for writing.
    #[inline]
    #[must_use]
    pub fn writing_enabled(&self) -> bool {
        match self {
            #[cfg(feature = "geojson")]
            OsmFormat::GeoJson => true,
            #[cfg(feature = "json")]
            OsmFormat::Json => true,
            #[cfg(feature = "json")]
            OsmFormat::Overpass => true,
            #[cfg(feature = "o5m")]
            OsmFormat::O5m => true,
            #[cfg(feature = "opl")]
            OsmFormat::Opl => true,
            #[cfg(feature = "xml")]
            OsmFormat::Xml => true,
            _ => false,
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

            match ext.as_str() {
                "geojson" => Some(OsmFormat::GeoJson),
                "json" => Some(OsmFormat::Json),
                "o5m" => Some(OsmFormat::O5m),
                "opl" => Some(OsmFormat::Opl),
                "osm" | "xml" => Some(OsmFormat::Xml),
                "pbf" => Some(OsmFormat::Pbf),
                _ => None,
            }
        }

        inner(ext.as_ref())
    }
}

impl fmt::Display for OsmFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OsmFormat::GeoJson => write!(f, "geojson")?,
            OsmFormat::Json => write!(f, "json")?,
            OsmFormat::Overpass => write!(f, "overpass")?,
            OsmFormat::O5m => write!(f, "o5m")?,
            OsmFormat::Opl => write!(f, "opl")?,
            OsmFormat::Xml => write!(f, "xml")?,
            OsmFormat::Pbf => write!(f, "pbf")?,
        }

        Ok(())
    }
}
