use std::cell::UnsafeCell;
use std::str::FromStr;

use crate::SkywayError;

#[derive(Debug)]
pub struct LazyOsmNumeric<T: FromStr + PartialEq, const NUM_EXPECTED_DIGITS: u8> {
    original_string: String,
    parsed_value: UnsafeCell<Option<T>>,
}

impl<T: FromStr + PartialEq, const NUM_EXPECTED_DIGITS: u8> LazyOsmNumeric<T, NUM_EXPECTED_DIGITS> {
    pub fn parse(&self) -> &T {
        unsafe {
            if (*self.parsed_value.get()).is_none() {
                match self.original_string.parse::<T>() {
                    Ok(v) => {
                        *self.parsed_value.get() = Some(v);
                    }
                    Err(_) => panic!("Unable to parse numeric string as number"),
                }
            }
            (*self.parsed_value.get()).as_ref().unwrap()
        }
    }
    pub fn as_str(&self) -> &str {
        &self.original_string
    }
    pub fn to_string(self) -> String {
        self.into()
    }
}

impl<const NUM_EXPECTED_DIGITS: u8> TryInto<LazyOsmNumeric<f64, NUM_EXPECTED_DIGITS>> for &str {
    type Error = SkywayError;

    fn try_into(self) -> Result<LazyOsmNumeric<f64, NUM_EXPECTED_DIGITS>, Self::Error> {
        match self.parse::<f64>() {
            Ok(_) => Ok(LazyOsmNumeric {
                original_string: self.to_owned(),
                parsed_value: UnsafeCell::new(None),
            }),
            Err(_) => Err(SkywayError::InvalidInputFile),
        }
    }
}

impl<const NUM_EXPECTED_DIGITS: u8> TryInto<LazyOsmNumeric<f64, NUM_EXPECTED_DIGITS>> for &[u8] {
    type Error = SkywayError;

    fn try_into(self) -> Result<LazyOsmNumeric<f64, NUM_EXPECTED_DIGITS>, Self::Error> {
        let mut saw_digit = false;
        let mut i = 0;

        // handle optional minus sign
        if i < self.len() && (self[i] == b'-') {
            i += 1;
        }

        // handle digits and one optional decimal point
        let mut saw_decimal = false;
        while i < self.len() {
            match self[i] {
                b'0'..=b'9' => saw_digit = true,
                b'.' if !saw_decimal => saw_decimal = true,
                _ => return Err(SkywayError::InvalidInputFile),
            }
            i += 1;
        }

        if !saw_digit {
            return Err(SkywayError::InvalidInputFile);
        }

        // unsafe because we have manually checked each byte as valid UTF-8
        let string = unsafe { String::from_utf8_unchecked(self.to_vec()) };

        Ok(LazyOsmNumeric {
            original_string: string,
            parsed_value: UnsafeCell::new(None),
        })
    }
}

impl<T: FromStr + PartialEq, const NUM_EXPECTED_DIGITS: u8> Into<String>
    for LazyOsmNumeric<T, NUM_EXPECTED_DIGITS>
{
    fn into(self) -> String {
        self.original_string
    }
}

impl<T: FromStr + PartialEq, const NUM_EXPECTED_DIGITS: u8> PartialEq
    for LazyOsmNumeric<T, NUM_EXPECTED_DIGITS>
{
    fn eq(&self, other: &Self) -> bool {
        self.parse().eq(other.parse())
    }
    fn ne(&self, other: &Self) -> bool {
        self.parse().ne(&other.parse())
    }
}
