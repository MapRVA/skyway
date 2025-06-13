use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use rayon::prelude::*;
use std::collections::VecDeque;
use std::io::BufRead;
use std::mem;

use std::path::PathBuf;
use std::str::from_utf8;
use std::sync::mpsc::Sender;

use crate::SkywayError;
use crate::elements::{Element, ElementBuilder, ElementTypeBuilder, Member};
use crate::{
    chunks::{ChunkBuilder, ElementChunk},
    elements::{Metadata, SimpleElementType},
    readers::Reader,
};

use super::coord_from_f64;

/// XML-specific reading errors
#[derive(Debug)]
enum XmlReadError {
    MissingAttribute(String),
    InvalidAttributeValue { attr: String, value: String },
    ParsingError(String),
    UnexpectedElement(String),
}

impl From<XmlReadError> for SkywayError {
    fn from(err: XmlReadError) -> SkywayError {
        match err {
            XmlReadError::MissingAttribute(attr) => SkywayError::InvalidInputFile,
            // FIXME: InvalidInputFile should support a message
            // Once that fix is made across the rest of the library,
            // I'll add better messages here
            XmlReadError::InvalidAttributeValue { attr, value } => SkywayError::UnexpectedError(
                format!("Invalid value '{}' for attribute '{}'", value, attr),
            ),
            XmlReadError::ParsingError(msg) => SkywayError::InvalidInputFile,
            XmlReadError::UnexpectedElement(elem) => SkywayError::InvalidInputFile,
        }
    }
}

// Convert a BytesStart event for an <osm> tag into a Metadata object
fn generate_metadata(osm_event: BytesStart) -> Metadata {
    let mut metadata = Metadata::default();

    for attr in osm_event.attributes() {
        if let Ok(attr) = attr {
            match attr.key.into_inner() {
                b"version" => {
                    metadata.version = attr_value_to_str(attr.value.as_ref())
                        .ok()
                        .map(|s| s.to_string())
                }

                b"generator" => {
                    metadata.generator = attr_value_to_str(attr.value.as_ref())
                        .ok()
                        .map(|s| s.to_string());
                }
                // TODO: Add other metadata attributes...
                _ => {}
            }
        }
    }
    metadata
}

struct ElementBuffer {
    builder: ChunkBuilder,
    current_elements: Vec<Element>,
    completed_chunks: VecDeque<ElementChunk>,
}

impl ElementBuffer {
    fn new(builder: ChunkBuilder) -> Self {
        Self {
            current_elements: Vec::with_capacity(builder.max_size),
            completed_chunks: VecDeque::new(),
            builder,
        }
    }

    // Add a new element to the buffer, potentially creating new chunks
    fn push(&mut self, element: Element) {
        self.current_elements.push(element);

        if self.current_elements.len() >= self.builder.max_size {
            self.flush();
        }
    }

    // Force creation of a chunk from current elements
    fn flush(&mut self) {
        if !self.current_elements.is_empty() {
            let elements = std::mem::replace(
                &mut self.current_elements,
                Vec::with_capacity(self.builder.max_size),
            );
            let chunk = self.builder.build_next_chunk(elements.into_boxed_slice());
            self.completed_chunks.push_back(chunk);
        }
    }

    // Convert buffer into a parallel iterator
    fn into_iter(mut self) -> impl ParallelIterator<Item = ElementChunk> {
        self.flush();
        self.completed_chunks.into_iter().par_bridge()
    }
}

struct ParseMachine {
    read_buffer: Vec<u8>,
    reader: quick_xml::reader::Reader<Box<dyn BufRead + Send>>,
    element_buffer: ElementBuffer,
    element_builder: ElementBuilder,
    in_osm: bool,
    current_element: Option<SimpleElementType>,
    metadata_sender: Sender<Metadata>,
}

impl ParseMachine {
    fn new(
        reader: quick_xml::reader::Reader<Box<dyn BufRead + Send>>,
        chunk_builder: ChunkBuilder,
        metadata_sender: Sender<Metadata>,
    ) -> Self {
        Self {
            read_buffer: Vec::new(),
            reader,
            element_buffer: ElementBuffer::new(chunk_builder),
            element_builder: ElementBuilder::default(),
            in_osm: false,
            current_element: None,
            metadata_sender,
        }
    }

    fn handle_tag(&mut self, tag: BytesStart) -> Result<(), SkywayError> {
        let mut k = None;
        let mut v = None;

        for attr in tag.attributes() {
            let attr = attr.map_err(|e| XmlReadError::ParsingError(e.to_string()))?;
            match attr.key.into_inner() {
                b"k" => k = attr_value_to_str(attr.value.as_ref())?.parse().ok(),
                b"v" => v = attr_value_to_str(attr.value.as_ref())?.parse().ok(),
                _ => {}
            }
        }

        if let (Some(key), Some(value)) = (k, v) {
            self.element_builder.tags.insert(key, value);
        }

        Ok(())
    }

    fn process_event(&mut self, event: Event) -> Result<(), SkywayError> {
        match event {
            Event::Start(s) => match s.name() {
                QName(b"osm") => {
                    self.in_osm = true;
                    self.metadata_sender
                        .send(generate_metadata(s))
                        .expect("Unable to send metadata out of reader process");
                    Ok(())
                }
                QName(b"node") | QName(b"way") | QName(b"relation") => {
                    if let Ok(Some(element_type)) = self.start_element(s) {
                        self.current_element = Some(element_type);
                    }
                    Ok(())
                }
                _ => Ok(()),
            },
            Event::End(_) => {
                if let Ok(Some(element)) = self.finish_element() {
                    self.element_buffer.push(element);
                    self.current_element = None;
                }
                Ok(())
            }
            Event::Empty(e) => match e.name() {
                QName(b"node") => {
                    // <node ... /> elements are registered as "empty" because they
                    // don't have separate <node> and </node> events.
                    // FIXME: make this more concise, this logic is not all necessary, we know it's a node
                    if let Ok(Some(element_type)) = self.start_element(e) {
                        self.current_element = Some(element_type);
                    }
                    if let Ok(Some(element)) = self.finish_element() {
                        self.element_buffer.push(element);
                        self.current_element = None;
                    }
                    Ok(())
                }
                QName(b"tag") => self.handle_tag(e),
                _ => self.handle_empty_element(e),
            },
            _ => Ok(()),
        }
    }

    fn into_chunk_iter(
        mut self,
    ) -> Result<impl ParallelIterator<Item = ElementChunk>, SkywayError> {
        loop {
            let event = self
                .reader
                .read_event_into(&mut self.read_buffer)
                .map_err(|e| {
                    XmlReadError::ParsingError(format!(
                        "Error at position {}: {:?}",
                        self.reader.buffer_position(),
                        e
                    ))
                })?;

            // Convert event to owned data that doesn't reference the buffer
            let owned_event = event.into_owned();

            // Check for EOF before processing
            if let Event::Eof = owned_event {
                break;
            }

            // Now we can process the owned event
            self.process_event(owned_event)?;

            // Clear the buffer after we're done
            self.read_buffer.clear();
        }

        // Verify we found and processed an OSM document
        if !self.in_osm {
            return Err(XmlReadError::ParsingError("No OSM document found".to_string()).into());
        }

        // Ensure any remaining elements are processed
        self.element_buffer.flush();

        Ok(self.element_buffer.into_iter())
    }

    fn start_element(
        &mut self,
        start: BytesStart,
    ) -> Result<Option<SimpleElementType>, XmlReadError> {
        for attr in start.attributes() {
            let attr = attr.map_err(|e| XmlReadError::ParsingError(e.to_string()))?;
            match attr.key.into_inner() {
                b"id" => {
                    self.element_builder.id = attr_value_to_str(attr.value.as_ref())?.parse().ok()
                }
                b"version" => {
                    self.element_builder.version =
                        attr_value_to_str(attr.value.as_ref())?.parse().ok()
                }
                b"timestamp" => {
                    self.element_builder.timestamp =
                        Some(attr_value_to_str(attr.value.as_ref())?.to_string())
                }
                b"changeset" => {
                    self.element_builder.changeset =
                        attr_value_to_str(attr.value.as_ref())?.parse().ok()
                }
                b"uid" => {
                    self.element_builder.uid = attr_value_to_str(attr.value.as_ref())?.parse().ok()
                }
                b"user" => {
                    self.element_builder.user =
                        Some(attr_value_to_str(attr.value.as_ref())?.to_string())
                }
                b"visible" => {
                    self.element_builder.visible =
                        Some(attr_value_to_str(attr.value.as_ref())? == "true")
                }
                _ => (),
            }
        }

        let element_type = match start.name().into_inner() {
            b"node" => {
                let mut lat = None;
                let mut lon = None;

                for attr in start.attributes() {
                    let attr = attr.map_err(|e| XmlReadError::ParsingError(e.to_string()))?;
                    match attr.key.into_inner() {
                        b"lat" => lat = attr_value_to_str(attr.value.as_ref())?.parse().ok(),
                        b"lon" => lon = attr_value_to_str(attr.value.as_ref())?.parse().ok(),
                        _ => {}
                    }
                }

                // Required attributes for nodes
                if lat.is_none() || lon.is_none() {
                    return Err(XmlReadError::MissingAttribute("lat/lon".to_string()));
                }

                self.element_builder.element_type = Some(ElementTypeBuilder::NodeBuilder {
                    lat: lat.map(|f| coord_from_f64(&f)),
                    lon: lon.map(|f| coord_from_f64(&f)),
                });

                Some(SimpleElementType::Node)
            }
            b"way" => {
                self.element_builder.element_type =
                    Some(ElementTypeBuilder::WayBuilder { nodes: Vec::new() });

                Some(SimpleElementType::Way)
            }
            b"relation" => {
                self.element_builder.element_type = Some(ElementTypeBuilder::RelationBuilder {
                    members: Vec::new(),
                });

                Some(SimpleElementType::Relation)
            }
            _ => None,
        };

        Ok(element_type)
    }

    fn finish_element(&mut self) -> Result<Option<Element>, SkywayError> {
        if self.element_builder.element_type.is_some() && self.element_builder.id.is_some() {
            let temp_builder = mem::replace(&mut self.element_builder, ElementBuilder::default());
            match temp_builder.build() {
                element @ Element { .. } => Ok(Some(element)),
                #[allow(unreachable_patterns)]
                _ => Err(XmlReadError::ParsingError("Failed to build element".to_string()).into()),
            }
        } else {
            Ok(None)
        }
    }

    fn handle_empty_element(&mut self, empty: BytesStart) -> Result<(), SkywayError> {
        match empty.name().into_inner() {
            b"nd" => {
                if let Some(SimpleElementType::Way) = self.current_element {
                    if let Some(ElementTypeBuilder::WayBuilder { nodes }) =
                        &mut self.element_builder.element_type
                    {
                        for attr in empty.attributes() {
                            let attr =
                                attr.map_err(|e| XmlReadError::ParsingError(e.to_string()))?;
                            if attr.key.into_inner() == b"ref" {
                                if let Ok(node_ref) =
                                    attr_value_to_str(attr.value.as_ref())?.parse::<i64>()
                                {
                                    nodes.push(node_ref);
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
            b"member" => {
                if let Some(SimpleElementType::Relation) = self.current_element {
                    if let Some(ElementTypeBuilder::RelationBuilder { members }) =
                        &mut self.element_builder.element_type
                    {
                        let mut role = None;
                        let mut ref_id = None;
                        let mut type_ = None;

                        for attr in empty.attributes() {
                            let attr =
                                attr.map_err(|e| XmlReadError::ParsingError(e.to_string()))?;
                            match attr.key.into_inner() {
                                b"role" => {
                                    role = Some(attr_value_to_str(attr.value.as_ref())?.to_string())
                                }
                                b"ref" => {
                                    ref_id = attr_value_to_str(attr.value.as_ref())?.parse().ok()
                                }
                                b"type" => {
                                    type_ = match attr_value_to_str(attr.value.as_ref())? {
                                        "node" => Some(SimpleElementType::Node),
                                        "way" => Some(SimpleElementType::Way),
                                        "relation" => Some(SimpleElementType::Relation),
                                        _ => None,
                                    }
                                }
                                _ => {}
                            }
                        }

                        if let (Some(id), Some(t)) = (ref_id, type_) {
                            members.push(Member {
                                role,
                                id,
                                t: Some(t),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn attr_value_to_str(value: &[u8]) -> Result<&str, XmlReadError> {
    from_utf8(value).map_err(|e| XmlReadError::ParsingError(e.to_string()))
}

#[derive(Clone)]
pub struct XmlReader {}

impl XmlReader {
    pub fn new() -> Self {
        XmlReader {}
    }
}

impl Reader for XmlReader {
    fn read_file(
        self,
        src: Option<PathBuf>,
        metadata_sender: Sender<Metadata>,
        chunk_builder: ChunkBuilder,
    ) -> impl ParallelIterator<Item = ElementChunk> {
        let reader = quick_xml::reader::Reader::from_reader(super::get_reader(src));
        let parse_machine = ParseMachine::new(reader, chunk_builder, metadata_sender);
        match parse_machine.into_chunk_iter() {
            Ok(result) => result,
            Err(e) => {
                panic!("Failed to parse XML: {}", e)
            }
        }
    }
}
