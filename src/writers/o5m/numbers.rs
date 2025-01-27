use bit_vec::BitVec;
use chrono::DateTime;

use crate::elements::SimpleElementType;

/// newtype for o5m's "signed integers"
#[derive(Clone)]
pub struct SignedInteger(Vec<u8>);

impl From<SignedInteger> for Vec<u8> {
    fn from(value: SignedInteger) -> Self {
        value.0
    }
}

impl IntoIterator for SignedInteger {
    type Item = u8;
    type IntoIter = std::vec::IntoIter<u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<i64> for SignedInteger {
    fn from(value: i64) -> Self {
        if value == 0 {
            SignedInteger(vec![0x00])
        } else if value.is_positive() {
            SignedInteger(convert_number(&value.to_be_bytes(), SignBit::Positive))
        } else {
            SignedInteger(convert_number(
                &(-value - 1).to_be_bytes(),
                SignBit::Negative,
            ))
        }
    }
}

impl From<i32> for SignedInteger {
    fn from(value: i32) -> Self {
        if value == 0 {
            SignedInteger(vec![0x00])
        } else if value.is_positive() {
            SignedInteger(convert_number(&value.to_be_bytes(), SignBit::Positive))
        } else {
            SignedInteger(convert_number(
                &(-value - 1).to_be_bytes(),
                SignBit::Negative,
            ))
        }
    }
}

/// newtype for o5m's "unsigned integers"
#[derive(Clone)]
pub struct UnsignedInteger(Vec<u8>);

impl From<UnsignedInteger> for Vec<u8> {
    fn from(value: UnsignedInteger) -> Self {
        value.0
    }
}

impl IntoIterator for UnsignedInteger {
    type Item = u8;
    type IntoIter = std::vec::IntoIter<u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<i32> for UnsignedInteger {
    fn from(value: i32) -> Self {
        UnsignedInteger(convert_number(&value.to_be_bytes(), SignBit::None))
    }
}

impl From<i64> for UnsignedInteger {
    fn from(value: i64) -> Self {
        UnsignedInteger(convert_number(&value.to_be_bytes(), SignBit::None))
    }
}

impl From<usize> for UnsignedInteger {
    fn from(value: usize) -> Self {
        UnsignedInteger(convert_number(&value.to_be_bytes(), SignBit::None))
    }
}

pub enum SignBit {
    Positive,
    Negative,
    None,
}

pub fn convert_number(bytes: &[u8], sign_bit: SignBit) -> Vec<u8> {
    // add five extra zeroes to the front of the BitVec
    //
    // TODO: prove that this is the lowest number of leading
    // zeroes necessary to prevent an overflowing subtraction
    // at the line `split_index = bit_vec.len() - 7;` below
    let mut bit_vec = BitVec::from_elem(5, false);

    // append the actual input bytes to our BitVec
    bit_vec.append(&mut BitVec::from_bytes(bytes));

    // if a sign bit was passed, add it to the end of the BitVec
    // (least significant bit of least significant byte)
    match sign_bit {
        SignBit::Positive => bit_vec.push(false),
        SignBit::Negative => bit_vec.push(true),
        SignBit::None => (),
    }

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

pub fn convert_index(index: usize) -> Vec<u8> {
    convert_number(&index.to_be_bytes(), SignBit::None)
}

pub struct DeltaCoder {
    last_changeset: i64,
    last_id: i64,
    last_lat: i32,
    last_lon: i32,
    last_rel_ref_n: i64,
    last_rel_ref_w: i64,
    last_rel_ref_r: i64,
    last_timestamp: i64,
    last_way_ref: i64,
}

impl DeltaCoder {
    pub fn new() -> Self {
        DeltaCoder {
            last_changeset: 0.into(),
            last_id: 0.into(),
            last_lat: 0.into(),
            last_lon: 0.into(),
            last_rel_ref_n: 0.into(),
            last_rel_ref_w: 0.into(),
            last_rel_ref_r: 0.into(),
            last_timestamp: 0.into(),
            last_way_ref: 0.into(),
        }
    }

    pub fn hit_changeset(&mut self, value: i64) -> SignedInteger {
        let delta = value - self.last_changeset;
        self.last_changeset = value;
        delta.into()
    }

    pub fn hit_id(&mut self, value: i64) -> SignedInteger {
        let delta = value - self.last_id;
        self.last_id = value;
        delta.into()
    }

    pub fn hit_lat(&mut self, value: f64) -> SignedInteger {
        let nanodegrees = (value * 1e7) as i32;
        let delta = nanodegrees.overflowing_sub(self.last_lat).0;
        self.last_lat = nanodegrees;
        delta.into()
    }

    pub fn hit_lon(&mut self, value: f64) -> SignedInteger {
        let nanodegrees = (value * 1e7) as i32;
        let delta = nanodegrees.overflowing_sub(self.last_lon).0;
        self.last_lon = nanodegrees;
        delta.into()
    }

    pub fn hit_rel_ref(&mut self, element_type: &SimpleElementType, value: i64) -> SignedInteger {
        let last_ref = match element_type {
            SimpleElementType::Node => &mut self.last_rel_ref_n,
            SimpleElementType::Way => &mut self.last_rel_ref_w,
            SimpleElementType::Relation => &mut self.last_rel_ref_r,
        };

        let delta = value - *last_ref;
        *last_ref = value;
        delta.into()
    }

    pub fn hit_timestamp(&mut self, value: &str) -> SignedInteger {
        let datetime = DateTime::parse_from_rfc3339(value).unwrap();
        let seconds = datetime.timestamp();
        let delta = seconds - self.last_timestamp;
        self.last_timestamp = seconds;
        delta.into()
    }

    pub fn hit_way_ref(&mut self, value: i64) -> SignedInteger {
        let delta = value - self.last_way_ref;
        self.last_way_ref = value;
        delta.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_number() {
        assert_eq!(convert_number(&vec![0x05], SignBit::None), vec![0x05]);

        assert_eq!(convert_number(&vec![0x7f], SignBit::None), vec![0x7f]);

        assert_eq!(
            convert_number(&vec![0x01, 0x43], SignBit::None),
            vec![0xc3, 0x02]
        );

        assert_eq!(
            convert_number(&vec![0x40, 0x00], SignBit::None),
            vec![0x80, 0x80, 0x01]
        );
    }

    #[test]
    fn test_unsigned_integer_from_i64() {
        let input1: i64 = 5;
        let expected1 = vec![0x05];
        assert_eq!(UnsignedInteger::from(input1).0, expected1);

        let input2: i64 = 127;
        let expected2 = vec![0x7f];
        assert_eq!(UnsignedInteger::from(input2).0, expected2);

        let input3: i64 = 323;
        let expected3 = vec![0xc3, 0x02];
        assert_eq!(UnsignedInteger::from(input3).0, expected3);

        let input4: i64 = 16384;
        let expected4 = vec![0x80, 0x80, 0x01];
        assert_eq!(UnsignedInteger::from(input4).0, expected4);
    }

    #[test]
    fn test_unsigned_integer_from_i32() {
        let input1: i32 = 5;
        let expected1 = vec![0x05];
        assert_eq!(UnsignedInteger::from(input1).0, expected1);

        let input2: i32 = 127;
        let expected2 = vec![0x7f];
        assert_eq!(UnsignedInteger::from(input2).0, expected2);

        let input3: i32 = 323;
        let expected3 = vec![0xc3, 0x02];
        assert_eq!(UnsignedInteger::from(input3).0, expected3);

        let input4: i32 = 16384;
        let expected4 = vec![0x80, 0x80, 0x01];
        assert_eq!(UnsignedInteger::from(input4).0, expected4);
    }

    #[test]
    fn test_signed_integer_from_i64() {
        let input1: i64 = 4;
        let expected1 = vec![0x08];
        assert_eq!(SignedInteger::from(input1).0, expected1);

        let input2: i64 = 64;
        let expected2 = vec![0x80, 0x01];
        assert_eq!(SignedInteger::from(input2).0, expected2);

        let input3: i64 = -2;
        let expected3 = vec![0x03];
        assert_eq!(SignedInteger::from(input3).0, expected3);

        let input4: i64 = -3;
        let expected4 = vec![0x05];
        assert_eq!(SignedInteger::from(input4).0, expected4);

        let input5: i64 = -65;
        let expected5 = vec![0x81, 0x01];
        assert_eq!(SignedInteger::from(input5).0, expected5);

        let input6: i64 = 0;
        let expected6 = vec![0x00];
        assert_eq!(SignedInteger::from(input6).0, expected6);
    }

    #[test]
    fn test_signed_integer_from_i32() {
        let input1: i32 = 4;
        let expected1 = vec![0x08];
        assert_eq!(SignedInteger::from(input1).0, expected1);

        let input2: i32 = 64;
        let expected2 = vec![0x80, 0x01];
        assert_eq!(SignedInteger::from(input2).0, expected2);

        let input3: i32 = -2;
        let expected3 = vec![0x03];
        assert_eq!(SignedInteger::from(input3).0, expected3);

        let input4: i32 = -3;
        let expected4 = vec![0x05];
        assert_eq!(SignedInteger::from(input4).0, expected4);

        let input5: i32 = -65;
        let expected5 = vec![0x81, 0x01];
        assert_eq!(SignedInteger::from(input5).0, expected5);

        let input6: i32 = 0;
        let expected6 = vec![0x00];
        assert_eq!(SignedInteger::from(input6).0, expected6);
    }

    #[test]
    fn test_delta_coder() {
        let mut delta_coder = DeltaCoder::new();

        // first node
        assert_eq!(delta_coder.hit_id(125799 as i64).0, vec![0xce, 0xad, 0x0f]);
        assert_eq!(
            delta_coder.hit_timestamp("2010-09-30T19:23:30Z").0,
            vec![0xe4, 0x8e, 0xa7, 0xca, 0x09],
        );
        assert_eq!(
            delta_coder.hit_changeset(5922698 as i64).0,
            vec![0x94, 0xfe, 0xd2, 0x05],
        );
        assert_eq!(
            delta_coder.hit_lat(53.0749606 as f64).0,
            vec![0xcc, 0xe2, 0x94, 0xfa, 0x03],
        );
        assert_eq!(
            delta_coder.hit_lon(8.7867843 as f64).0,
            vec![0x86, 0x87, 0xe6, 0x53],
        );

        // second node (each hit requires calculating a delta)
        assert_eq!(delta_coder.hit_id(125800).0, vec![0x02]);
        assert_eq!(
            delta_coder.hit_timestamp("2010-09-30T19:57:15Z").0,
            vec![0xd2, 0x1f],
        );
        assert_eq!(
            delta_coder.hit_changeset(5923003 as i64).0,
            vec![0xe2, 0x04],
        );
        assert_eq!(
            delta_coder.hit_lat(53.0719347 as f64).0,
            vec![0xe5, 0xd8, 0x03],
        );
        assert_eq!(
            delta_coder.hit_lon(8.7840318 as f64).0,
            vec![0x89, 0xae, 0x03],
        );
    }
}
