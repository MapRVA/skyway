use chrono::DateTime;
use rayon::prelude::*;

use std::{fs::File, io::stdout, path::PathBuf, sync::mpsc::Receiver};

use crate::{
    chunks::ElementChunk,
    elements::{Element, ElementType, Metadata, SimpleElementType},
    SkywayError,
};

mod numbers;
use numbers::{DeltaCoder, SignedInteger, UnsignedInteger};

mod strings;
use strings::StringTable;

use super::Writer;

fn convert_element(
    element: Element,
    delta_coder: &mut DeltaCoder,
    string_table: &mut StringTable,
) -> Vec<u8> {
    let mut element_data: Vec<u8> = Vec::new();

    // write element id to output
    element_data.extend(delta_coder.hit_id(element.id));

    // write element metadata to output
    // TODO: allow for just version and timestamp, with no author info?
    if let (Some(version), Some(timestamp), Some(changeset), Some(uid), Some(user)) = (
        element.version,
        element.timestamp,
        element.changeset,
        element.uid,
        element.user,
    ) {
        element_data.extend(UnsignedInteger::from(version));
        element_data.extend(delta_coder.hit_timestamp(&timestamp));
        element_data.extend(delta_coder.hit_changeset(changeset));
        element_data.extend(string_table.hit_user(uid, user));
    } else {
        element_data.push(0x00);
    }

    match &element.element_type {
        ElementType::Node { lat, lon } => {
            element_data.extend(delta_coder.hit_lon(*lon));
            element_data.extend(delta_coder.hit_lat(*lat));
        }
        ElementType::Way { nodes } => {
            let mut node_refs = Vec::<u8>::new();

            // convert each node id reference to a signed integer
            for node in nodes {
                node_refs.extend(delta_coder.hit_way_ref(*node));
            }

            // add length of references to output
            element_data.extend(UnsignedInteger::from(node_refs.len()));

            // add references to output
            element_data.extend(node_refs);
        }
        ElementType::Relation { members } => {
            let mut refs = Vec::new();

            let mut current_member_type: &SimpleElementType;
            for member in members {
                match &member.t {
                    Some(t) => {
                        current_member_type = t;
                    }
                    // FIXME: somehow warn the user sooner, or try to determine the type of this member?
                    None => panic!("Relation member types must be annotated to output o5m"),
                }

                // write member id (delta-coded) to output
                // o5m delta-codes per-type of member, across all relations
                refs.extend(delta_coder.hit_rel_ref(current_member_type, member.id));

                // write member role to output
                refs.extend(string_table.hit_rel_ref(current_member_type, &member.role))
            }

            // add length of references to output
            element_data.extend(UnsignedInteger::from(refs.len()));

            // add references to output
            element_data.extend(refs);
        }
    }

    // append tags to output
    for tag in element.tags {
        element_data.extend(string_table.hit_tag(&tag.0, &tag.1));
    }

    // new Vec<u8> for the final output
    let mut output: Vec<u8> = Vec::new();

    // add special element type code to final output
    match element.element_type {
        ElementType::Node { .. } => output.push(0x10),
        ElementType::Way { .. } => output.push(0x11),
        ElementType::Relation { .. } => output.push(0x12),
    }

    // write the length of our element data to final output
    output.extend(UnsignedInteger::from(element_data.len()));

    // now, append all the element data to final output
    output.extend(element_data);

    output
}

// struct to hold elements that need to be held
// before writing because the output format
// requires that they are sorted
struct WaitingElements {
    nodes: Vec<Element>,
    ways: Vec<Element>,
    relations: Vec<Element>,
}

impl WaitingElements {
    fn append(&mut self, element: Element) {
        match element.element_type {
            ElementType::Node { .. } => self.nodes.push(element),
            ElementType::Way { .. } => self.ways.push(element),
            ElementType::Relation { .. } => self.relations.push(element),
        }
    }

    fn new() -> Self {
        return WaitingElements {
            nodes: Vec::new(),
            ways: Vec::new(),
            relations: Vec::new(),
        };
    }

    // sort each of the Vecs by element ID, ascending order
    fn sort(&mut self) {
        self.nodes.sort_by(|a, b| a.id.cmp(&b.id));
        self.ways.sort_by(|a, b| a.id.cmp(&b.id));
        self.relations.sort_by(|a, b| a.id.cmp(&b.id));
    }
}

impl Iterator for WaitingElements {
    type Item = Element;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.nodes.is_empty() {
            self.nodes.pop()
        } else if !self.ways.is_empty() {
            self.ways.pop()
        } else if !self.relations.is_empty() {
            self.relations.pop()
        } else {
            None
        }
    }
}

fn write_output<I>(
    metadata_receiver: Receiver<Metadata>,
    par_iter: I,
    mut dest: impl std::io::Write,
) where
    I: IntoParallelIterator<Item = ElementChunk>,
{
    let mut waiting_elements = WaitingElements::new();

    let metadata = metadata_receiver.into_iter().next();

    let chunks: Vec<ElementChunk> = par_iter.into_par_iter().collect();

    for chunk in chunks {
        for element in chunk.content {
            waiting_elements.append(element);
        }
    }

    // sort our container of waiting elements
    waiting_elements.sort();

    let mut delta_coder = DeltaCoder::new();
    let mut string_table = StringTable::new();

    // write starting byte to output
    dest.write(&vec![0xff])
        .expect("Unable to begin writing to output.");

    // TODO: write bounding box to dest, once skyway supports bounding boxes

    // write header to output
    dest.write(&vec![0xe0, 0x04, 0x6f, 0x35, 0x6d, 0x32])
        .expect("Unable to write header to o5m output.");

    // write file timestamp to output, if there is one
    if let Some(m) = metadata {
        if let Some(t) = m.timestamp {
            match DateTime::parse_from_rfc3339(&t) {
                Ok(d) => {
                    // byte that signals start of timestamp dataset
                    dest.write(&vec![0xdc])
                        .expect("Unable to write pre-timestamp byte to output.");

                    // calculate timestamp bytes
                    let timestamp_bytes = &Vec::<u8>::from(SignedInteger::from(d.timestamp()));

                    // write length of timestamp dataset to output
                    dest.write(&Vec::<u8>::from(UnsignedInteger::from(
                        timestamp_bytes.len(),
                    )))
                    .expect("Unable to write length of timestamp dataset to output.");

                    // write the rest of timestamp dataset to output
                    dest.write(timestamp_bytes)
                        .expect("Unable to write timestamp to output.");
                }
                Err(_) => {
                    // FIXME: Do better datetime parsing upstream
                    println!("WARNING: Unable to parse input timestamp as datetime.")
                }
            }
        }
    }

    dest.write(&vec![0xff])
        .expect("Unable to write reset byte to output.");

    // tracks whether we should write reset byte to the output
    let mut last_vec_had_elements = false;

    for element_vec in [
        waiting_elements.nodes,
        waiting_elements.ways,
        waiting_elements.relations,
    ] {
        if element_vec.len() > 0 {
            if last_vec_had_elements {
                // reset both counters
                delta_coder = DeltaCoder::new();
                string_table = StringTable::new();

                // write reset byte to output
                dest.write(&vec![0xff])
                    .expect("Unable to write reset byte to output.");
            }
            for element in element_vec {
                dest.write(&convert_element(
                    element,
                    &mut delta_coder,
                    &mut string_table,
                ))
                .expect("Error while writing element to o5m output.");
            }
            // this element Vec did have at least one element, so
            // we should right a reset byte on the next iteration
            last_vec_had_elements = true;
        } else {
            // if this element Vec doesn't have any elements, it isn't
            // necessary to write a second reset byte to the output
            last_vec_had_elements = false;
        }
    }

    // write final byte to output
    dest.write(&vec![0xfe])
        .expect("Unable to write final byte to output.");
}

pub struct O5mWriter {}

impl O5mWriter {
    pub fn new() -> Self {
        O5mWriter {}
    }
}

impl Writer for O5mWriter {
    fn write<I>(
        &self,
        par_iter: I,
        metadata_receiver: Receiver<Metadata>,
        dest: Option<PathBuf>,
    ) -> Result<(), SkywayError>
    where
        I: IntoParallelIterator<Item = ElementChunk>,
    {
        match dest {
            None => write_output(metadata_receiver, par_iter, stdout()),
            Some(a) => match File::create(PathBuf::from(a)) {
                Ok(b) => write_output(metadata_receiver, par_iter, b),
                Err(e) => {
                    panic!("Unable to open output file: {e:?}");
                }
            },
        };

        Ok(())
    }
}
