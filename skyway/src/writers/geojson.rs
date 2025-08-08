use std::{collections::HashMap, fs::File, io::stdout, path::PathBuf, sync::mpsc::Receiver};

use super::geo::{TaggedGeometry, convert_chunks};
use geojson::{Feature, FeatureCollection, GeoJson, Geometry, JsonObject, JsonValue};

use super::Writer;
use crate::{SkywayError, chunks::ElementChunk, elements::Metadata};

fn properties_from_tags(tags: HashMap<String, String>) -> JsonObject {
    let mut json_object = JsonObject::new();
    for (key, value) in tags.iter() {
        json_object.insert(key.to_owned(), JsonValue::String(value.to_owned()));
    }
    json_object
}

impl From<TaggedGeometry> for Feature {
    fn from(value: TaggedGeometry) -> Self {
        Feature {
            bbox: None,
            geometry: Some(Geometry::from(&value.geometry)),
            id: None,
            properties: Some(properties_from_tags(value.tags)),
            foreign_members: None,
        }
    }
}

fn tagged_geometries_to_feature_collection(
    tagged_geometries: Vec<TaggedGeometry>,
) -> FeatureCollection {
    let mut features: Vec<Feature> = Vec::new();

    for tagged_geometry in tagged_geometries {
        features.push(tagged_geometry.into());
    }

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}

fn write_output(output: String, mut dest_buffer: impl std::io::Write) -> Result<(), SkywayError> {
    dest_buffer.write_all(output.as_bytes())?;
    Ok(())
}

pub struct GeoJsonWriter {}

impl GeoJsonWriter {
    pub fn new() -> Self {
        GeoJsonWriter {}
    }
}

impl Writer for GeoJsonWriter {
    fn write(
        &self,
        element_receiver: Receiver<ElementChunk>,
        metadata_receiver: Receiver<Metadata>,
        dest: Option<PathBuf>,
    ) -> Result<(), SkywayError> {
        let tagged_geometries = convert_chunks(element_receiver)?;
        let feature_collection = tagged_geometries_to_feature_collection(tagged_geometries);

        // TODO: add metadata

        let output = GeoJson::from(feature_collection).to_string();

        match dest {
            None => write_output(output, stdout()),
            Some(a) => {
                let file = File::create(PathBuf::from(a))?;
                write_output(output, file)
            }
        }
    }
}
