use crate::elements::SimpleElementType;

use super::numbers::{convert_index, convert_number, SignBit};

#[derive(Clone, Debug, PartialEq)]
pub struct StringPair(Vec<u8>);

impl IntoIterator for StringPair {
    type Item = u8;
    type IntoIter = std::vec::IntoIter<u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub struct StringTable {
    cached_tags: Vec<StringPair>,
}

impl StringTable {
    pub fn new() -> Self {
        StringTable {
            cached_tags: Vec::new(),
        }
    }

    fn hit_cache(&mut self, bytes: Vec<u8>) -> Vec<u8> {
        let str_pair = StringPair(bytes);

        // determine index of string in StringTable
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

    // convert a tag (surrounding both key and value with zero-bytes)
    // returns a reference to string table cache when appropriate
    pub fn hit_tag(&mut self, key: &str, value: &str) -> Vec<u8> {
        let mut output = Vec::new();
        output.push(0x00);
        output.extend(key.as_bytes());
        output.push(0x00);
        output.extend(value.as_bytes());
        output.push(0x00);

        if key.len() + value.len() > 250 {
            output
        } else {
            self.hit_cache(output)
        }
    }

    pub fn hit_rel_ref(
        &mut self,
        element_type: &SimpleElementType,
        role: &Option<String>,
    ) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(0x00);

        match element_type {
            SimpleElementType::Node => bytes.push(0x30),
            SimpleElementType::Way => bytes.push(0x31),
            SimpleElementType::Relation => bytes.push(0x32),
        };

        bytes.extend(
            match &role {
                Some(r) => r,
                None => "",
            }
            .as_bytes(),
        );

        bytes.push(0x00);

        self.hit_cache(bytes)
    }

    // convert a user id (i32) and username (String) into the
    // bit-packed specification for o5m, returned as a Vec of bytes (u8)
    pub fn hit_user(&mut self, uid: i32, username: String) -> Vec<u8> {
        let mut output = Vec::new();
        output.push(0x00);
        let uid_bytes = convert_number(&uid.to_be_bytes(), SignBit::None);
        output.extend(&uid_bytes);
        output.push(0x00);
        output.extend(username.as_bytes());
        output.push(0x00);

        if uid_bytes.len() + username.len() > 250 {
            output
        } else {
            self.hit_cache(output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_tag() {
        let mut string_table = StringTable::new();

        let input1 = ("oneway", "yes");
        let expected1 = vec![
            0x00, 0x6f, 0x6e, 0x65, 0x77, 0x61, 0x79, 0x00, 0x79, 0x65, 0x73, 0x00,
        ];
        assert_eq!(string_table.hit_tag(input1.0, input1.1), expected1);

        let input2 = ("atm", "no");
        let expected2 = vec![0x00, 0x61, 0x74, 0x6d, 0x00, 0x6e, 0x6f, 0x00];
        assert_eq!(string_table.hit_tag(input2.0, input2.1), expected2);

        // tags totalling 250 bytes (or less) get cached
        let input3 = ("a", "a".repeat(249));
        string_table.hit_tag(input3.0, &input3.1);
        let expected3 = vec![0x01];
        assert_eq!(string_table.hit_tag(input3.0, &input3.1), expected3);

        // tags totalling 251 bytes (or more) do not get cached
        let input4 = ("a", "a".repeat(250));
        string_table.hit_tag(input4.0, &input4.1);
        let mut expected4 = vec![0x00, 0x61, 0x00];
        expected4.extend(vec![0x61; 250]);
        expected4.push(0x00);
        assert_eq!(string_table.hit_tag(input4.0, &input4.1), expected4);
    }

    #[test]
    fn test_convert_user() {
        let mut string_table = StringTable::new();

        let input1: (i32, String) = (1020, String::from("John"));
        let expected1 = vec![0x00, 0xfc, 0x07, 0x00, 0x4a, 0x6f, 0x68, 0x6e, 0x00];
        assert_eq!(string_table.hit_user(input1.0, input1.1), expected1);

        // tags totalling 250 bytes (or less) get cached
        // (uid 1020 comes out to two bytes)
        let input2: (i32, String) = (1020, "a".repeat(248));
        string_table.hit_user(input2.0, input2.clone().1);
        let expected2 = vec![0x01];
        assert_eq!(string_table.hit_user(input2.0, input2.1), expected2);

        // tags totalling 251 bytes (or more) do not get cached
        // (uid 1020 comes out to two bytes)
        let input3: (i32, String) = (1020, "a".repeat(249));
        string_table.hit_user(input3.0, input3.clone().1);
        let mut expected3 = vec![0x00, 0xfc, 0x07, 0x00];
        expected3.extend(vec![0x61; 249]);
        expected3.push(0x00);
        assert_eq!(string_table.hit_user(input3.0, input3.1), expected3);
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
