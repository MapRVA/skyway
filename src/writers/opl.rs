use std::fmt::{Error, Write};
use std::sync::mpsc::Receiver;

use crate::elements::{Element, ElementType, Metadata, SimpleElementType};

// wrapper struct that implements std::fmt::Write for any type
// that implements std::io::Write, this allows us to use write!
// macros with types that implement std::io::Write
struct ToFmtWrite<T>(pub T);

impl<T> Write for ToFmtWrite<T>
where
    T: std::io::Write,
{
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|_| std::fmt::Error)
    }
}

// this list is from the Osmium OPL implementation
fn should_escape_char(input: char) -> bool {
    match input {
        '\u{0021}'..='\u{0024}' => false, // code points 33-36
        '\u{0026}'..='\u{002b}' => false, // code points 38-43
        '\u{002d}'..='\u{003c}' => false, // code points 45-60
        '\u{003e}'..='\u{003f}' => false, // code points 62-63
        '\u{0041}'..='\u{007e}' => false, // code points 65-126
        '\u{00a1}'..='\u{00ac}' => false, // code points 161-172
        '\u{00ae}'..='\u{05ff}' => false, // code points 174-1535
        _ => true,
    }
}

// escape a given char according to the OPL spec, and push
// it onto a mutable String reference
fn append_escaped_char(input: char, output: &mut String) {
    output.push('%');

    // get the UTF-8 code point of the character
    let code_point = input as u32;

    // format the code point as hexadecimal (lowercase)
    let hex = format!("{:x}", code_point);
    output.push_str(&hex);

    output.push('%');
}

// takes a String input, returns an escaped String output
fn escape_string(input: String) -> String {
    let mut output = String::new();
    for c in input.chars() {
        if should_escape_char(c) {
            append_escaped_char(c, &mut output);
        } else {
            output.push(c);
        }
    }
    output
}

fn serialize_chunk(chunk: Vec<Element>) -> Result<String, Error> {
    let mut output = String::new();
    for element in chunk {
        match element.element_type {
            ElementType::Node { .. } => {
                output.push('n');
            }
            ElementType::Way { .. } => {
                output.push('w');
            }
            ElementType::Relation { .. } => {
                output.push('r');
            }
        }
        let id = &element.id;
        write!(output, "{id}")?;

        if let Some(v) = element.version {
            write!(output, " v{v}")?;
        }

        if let Some(v) = element.visible {
            if v {
                output.push_str(" dV");
            } else {
                output.push_str(" dD");
            }
        }

        if let Some(c) = element.changeset {
            write!(output, " c{c}")?;
        }

        if let Some(t) = element.timestamp {
            write!(output, " t{t}")?;
        }

        if let Some(u) = element.uid {
            write!(output, " i{u}")?;
        }

        if let Some(u) = element.user {
            let escaped_username = escape_string(u);
            write!(output, " u{escaped_username}")?;
        }

        output.push_str(" T");
        let tags_out = element
            .tags
            .into_iter()
            .map(|(k, v)| (escape_string(k), escape_string(v)))
            .map(|(k, v)| format!("{k}={v}"))
            .reduce(|acc, s| format!("{acc},{s}"))
            .unwrap_or_default();
        write!(output, "{tags_out}")?;

        match element.element_type {
            ElementType::Node { lat, lon } => {
                write!(output, " x{lon} y{lat}")?;
            }
            ElementType::Way { nodes } => {
                output.push_str(" N");
                let out = nodes
                    .into_iter()
                    .map(|n| format!("n{}", n.to_string()))
                    .reduce(|acc, s| format!("{acc},{s}"))
                    .unwrap();
                write!(output, "{out}")?;
            }
            ElementType::Relation { members } => {
                output.push_str(" M");

                let out = members
                    .into_iter()
                    .map(|m| {
                        let mref = m.id;
                        let mrole = m.role;
                        if let Some(mrole) = mrole {
                            let element_type_char = match m.t {
                                Some(SimpleElementType::Node) => 'n',
                                Some(SimpleElementType::Way) => 'w',
                                Some(SimpleElementType::Relation) => 'r',
                                None => panic!("Member type is None"),
                            };
                            let escaped_member_role = escape_string(mrole);
                            format!("{element_type_char}{mref}@{escaped_member_role}")
                        } else {
                            // TODO: Determine role by finding the relevant element?
                            format!("{mref}")
                        }
                    })
                    .reduce(|acc, s| format!("{acc},{s}"))
                    .unwrap();
                write!(output, "{out}")?;
            }
        }
        output.push('\n');
    }
    Ok(output)
}

#[allow(unused_variables)]
pub fn write_opl<D: std::io::Write>(receiver: Receiver<Vec<Element>>, metadata: Metadata, dest: D) {
    let mut writer = ToFmtWrite(dest);
    for chunk in receiver {
        match serialize_chunk(chunk) {
            Ok(s) => match writer.write_str(s.as_str()) {
                Ok(_) => (),
                Err(e) => panic!("Error writing to output {e:?}"),
            },
            Err(e) => panic!("Error serializing output: {e:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_escaped_char() {
        let mut string1 = String::from("");
        append_escaped_char(' ', &mut string1);
        assert_eq!(string1, "%20%");

        let mut string2 = String::from("");
        append_escaped_char(',', &mut string2);
        assert_eq!(string2, "%2c%");

        let mut string3 = String::from("");
        append_escaped_char('😱', &mut string3);
        assert_eq!(string3, "%1f631%");

        let mut string4 = String::from("");
        append_escaped_char('𒄈', &mut string4);
        assert_eq!(string4, "%12108%");
    }

    #[test]
    fn test_should_escape_char() {
        let test_chars = vec![' ', '\n', ',', '=', '@', '%', '😱'];
        for c in test_chars {
            assert_eq!(should_escape_char(c), true);
        }
    }

    #[test]
    fn test_escape_string() {
        let string1 = String::from("A,B");
        assert_eq!(escape_string(string1), "A%2c%B");

        let string2 = String::from("ohmy😱goodness");
        assert_eq!(escape_string(string2), "ohmy%1f631%goodness");
    }
}
