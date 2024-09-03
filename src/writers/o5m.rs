use bit_vec::BitVec;
use std::io::Write;
use std::sync::mpsc::Receiver;

use crate::elements::{Element, ElementType, Metadata};

// convert a single string (surround with zero-bytes)
fn convert_string(input: &str) -> Vec<u8> {
    let mut output = Vec::new();
    output.push(0x00);
    output.extend(input.as_bytes());
    output.push(0x00);
    output
}

// convert a tag (surrounding both key and value with zero-bytes)
fn convert_tag(key: &str, value: &str) -> Vec<u8> {
    let mut output = Vec::new();
    output.push(0x00);
    output.extend(key.as_bytes());
    output.push(0x00);
    output.extend(value.as_bytes());
    output.push(0x00);
    output
}

fn convert_f64(input: f64) -> Vec<u8> {
    unimplemented!()
}

fn convert_number(bytes: &[u8]) -> Vec<u8> {
    let mut bit_vec = BitVec::from_bytes(bytes);
    let mut output: Vec<u8> = Vec::new();
    let mut split_index;
    let mut this_bit;
    let mut bit_vec_continues = !bit_vec.none();
    while bit_vec_continues {
        // grab the last 7 bits (the first bit of each byte is
        // a flag indicating if the number continues into the
        // next byte)
        split_index = bit_vec.len() - 7;

        // split off the last 7 bits using the index we
        // calculated above
        this_bit = bit_vec.split_off(split_index);

        // decide ahead of time if this while loop will continue,
        // so that we can decide the value of the continuation bit
        bit_vec_continues = !bit_vec.none();

        // convert the BitVec into a Vec of (1) u8 byte, bit-shift
        // it right by one, and then perform a bitwise OR against
        // either 00000000 or 10000000, if it's the last or not,
        // respectively. then, append it to the Vec of output bytes
        output.extend::<Vec<u8>>(
            this_bit
                .to_bytes()
                .into_iter()
                .map(|b: u8| b >> 1 | if bit_vec_continues { 0x80 } else { 0x00 })
                .collect(),
        );
    }
    output
}

// convert a signed 64-bit integer (i64) into the bit-packed
// specification for o5m, returned as a Vec of bytes (u8)
fn convert_i64_as_unsigned(input: i64) -> Vec<u8> {
    convert_number(&input.to_be_bytes())
}

// convert a user id (i32) and username (String) into the
// bit-packed specification for o5m, returned as a Vec of bytes (u8)
fn convert_user(uid: i32, username: String) -> Vec<u8> {
    let mut output = Vec::new();
    output.push(0x00);
    output.extend(convert_number(&uid.to_be_bytes()));
    output.push(0x00);
    output.extend(username.as_bytes());
    output.push(0x00);
    output
}

fn convert_index(index: usize) -> Vec<u8> {
    convert_number(&index.to_be_bytes())
}

#[derive(Clone, Debug, PartialEq)]
struct StringPair(Vec<u8>);

struct StringTable {
    cached_tags: Vec<StringPair>,
}

impl StringTable {
    fn hit_cache(&mut self, bytes: Vec<u8>) -> Vec<u8> {
        let str_pair = StringPair(bytes);

        // determine index of key, value in TagTable
        // tag_position will be Option<usize>
        let tag_position = self.cached_tags.iter().position(|b| b == &str_pair);

        // if key, value were found in TagTable, return
        // index converted to o5m byte sequence. Otherwise,
        // insert the new tag into the TagTable then return
        // its byte sequence
        match tag_position {
            // increment by 1 to accomodate zero-indexed vector
            Some(v) => convert_index(v + 1),
            None => {
                self.cached_tags.insert(0, str_pair.clone());
                self.cached_tags.truncate(15000);
                str_pair.0
            }
        }
    }
    fn new() -> Self {
        StringTable {
            cached_tags: Vec::new(),
        }
    }
}

pub fn write_o5m<D: Write>(receiver: Receiver<Element>, metadata: Metadata, mut dest: D) {
    let mut waiting_nodes = Vec::new();
    let mut waiting_ways = Vec::new();
    let mut waiting_relations = Vec::new();

    for element in receiver {
        match element.element_type {
            ElementType::Node { .. } => waiting_nodes.push(element),
            ElementType::Way { .. } => waiting_ways.push(element),
            ElementType::Relation { .. } => waiting_relations.push(element),
        }
    }
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_string() {
        let input = "1inner";
        let expected = vec![0x00, 0x31, 0x69, 0x6e, 0x6e, 0x65, 0x72, 0x00];
        assert_eq!(convert_string(input), expected);
    }
    #[test]
    fn test_convert_tag() {
        let input1 = ("oneway", "yes");
        let expected1 = vec![
            0x00, 0x6f, 0x6e, 0x65, 0x77, 0x61, 0x79, 0x00, 0x79, 0x65, 0x73, 0x00,
        ];
        assert_eq!(convert_tag(input1.0, input1.1), expected1);

        let input2 = ("atm", "no");
        let expected2 = vec![0x00, 0x61, 0x74, 0x6d, 0x00, 0x6e, 0x6f, 0x00];
        assert_eq!(convert_tag(input2.0, input2.1), expected2);
    }
    #[test]
    fn test_convert_id() {
        let input1: i64 = 5;
        let expected1 = vec![0x05];
        assert_eq!(convert_i64_as_unsigned(input1), expected1);

        let input2: i64 = 127;
        let expected2 = vec![0x7f];
        assert_eq!(convert_i64_as_unsigned(input2), expected2);

        let input3: i64 = 323;
        let expected3 = vec![0xc3, 0x02];
        assert_eq!(convert_i64_as_unsigned(input3), expected3);

        let input4: i64 = 16384;
        let expected4 = vec![0x80, 0x80, 0x01];
        assert_eq!(convert_i64_as_unsigned(input4), expected4);
    }
    #[test]
    fn test_convert_user() {
        let input1: (i32, String) = (1020, String::from("John"));
        let expected1 = vec![0x00, 0xfc, 0x07, 0x00, 0x4a, 0x6f, 0x68, 0x6e, 0x00];
        assert_eq!(convert_user(input1.0, input1.1), expected1);
    }

    #[test]
    fn test_string_table() {
        let mut string_table = StringTable::new();

        let vec1 = vec![
            0x00, 0x6f, 0x6e, 0x65, 0x77, 0x61, 0x79, 0x00, 0x79, 0x65, 0x73, 0x00,
        ];
        assert_eq!(string_table.hit_cache(vec1.clone()), vec1);

        let vec2 = vec![0x00, 0x61, 0x74, 0x6d, 0x00, 0x6e, 0x6f, 0x00];
        assert_eq!(string_table.hit_cache(vec2.clone()), vec2);

        assert_eq!(string_table.hit_cache(vec1.clone()), vec![0x02]);

        let vec3 = vec![0x00, 0xfc, 0x07, 0x00, 0x4a, 0x6f, 0x68, 0x6e, 0x00];
        assert_eq!(string_table.hit_cache(vec3.clone()), vec3);

        assert_eq!(string_table.hit_cache(vec2), vec![0x02]);

        assert_eq!(string_table.hit_cache(vec1), vec![0x03]);

        assert_eq!(string_table.hit_cache(vec3), vec![0x01]);
    }
}
