use std::ffi::OsStr;
use std::fmt;
use std::path::PathBuf;

use crate::SkywayError;

/// Enum that represents the different OSM file formats skyway supports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum OsmFormat {
    GeoJson,
    Json,
    O5m,
    Opl,
    OverpassQuery,
    Xml,
    Pbf,
}

impl OsmFormat {
    fn format_error(ext: &str) -> SkywayError {
        SkywayError::UnknownFormat(ext.to_string())
    }

    #[inline]
    pub fn from_identifier<S>(id: S) -> Option<Self>
    where
        S: AsRef<str>,
    {
        match id.as_ref() {
            "geojson" => Some(OsmFormat::GeoJson),
            "json" => Some(OsmFormat::Json),
            "o5m" => Some(OsmFormat::O5m),
            "opl" => Some(OsmFormat::Opl),
            "overpass-query" => Some(OsmFormat::OverpassQuery),
            "xml" | "osm" => Some(OsmFormat::Xml),
            "pbf" => Some(OsmFormat::Pbf),
            _ => None,
        }
    }

    pub fn parse(
        cli_format: Option<String>,
        file_path: &Option<PathBuf>,
    ) -> Result<Self, SkywayError> {
        if let Some(format_str) = cli_format {
            Self::from_identifier(&format_str).ok_or(Self::format_error(
                format!("Unknown format identifier: {}", format_str).as_str(),
            ))
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

    /// Returns `true` if skyway can read `OsmFormat`.
    pub const fn can_read(&self) -> bool {
        match self {
            OsmFormat::Json => true,
            OsmFormat::OverpassQuery => true,
            OsmFormat::Opl => true,
            OsmFormat::Xml => true,
            OsmFormat::Pbf => true,
            _ => false,
        }
    }

    /// Returns `true` if skyway can write `OsmFormat`.
    pub const fn can_write(&self) -> bool {
        match self {
            OsmFormat::GeoJson => true,
            OsmFormat::Json => true,
            OsmFormat::O5m => true,
            OsmFormat::Opl => true,
            OsmFormat::Xml => true,
            _ => false,
        }
    }

    /// Return the `OsmFormat`s which are enabled for reading (feature is enabled).
    #[inline]
    #[must_use]
    pub fn reading_enabled(&self) -> bool {
        match self {
            #[cfg(feature = "json")]
            OsmFormat::Json => true,
            #[cfg(feature = "overpass-queries")]
            OsmFormat::OverpassQuery => true,
            #[cfg(feature = "opl")]
            OsmFormat::Opl => true,
            #[cfg(feature = "xml")]
            OsmFormat::Xml => true,
            #[cfg(feature = "pbf")]
            OsmFormat::Pbf => true,
            _ => false,
        }
    }

    /// Return the `OsmFormat`s which are enabled for writing (feature is enabled).
    #[inline]
    #[must_use]
    pub fn writing_enabled(&self) -> bool {
        match self {
            #[cfg(feature = "geojson")]
            OsmFormat::GeoJson => true,
            #[cfg(feature = "json")]
            OsmFormat::Json => true,
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
                "skyway does not support reading format {:?}",
                input
            )));
        }

        if !output.can_write() {
            return Err(SkywayError::UnsupportedWrite(format!(
                "skyway does not support writing format {:?}",
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
                "overpassql" => Some(OsmFormat::OverpassQuery),
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
            OsmFormat::OverpassQuery => write!(f, "overpass-query")?,
            OsmFormat::O5m => write!(f, "o5m")?,
            OsmFormat::Opl => write!(f, "opl")?,
            OsmFormat::Xml => write!(f, "xml")?,
            OsmFormat::Pbf => write!(f, "pbf")?,
        }

        Ok(())
    }
}
