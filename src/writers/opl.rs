use lexical;
use rayon::prelude::*;
use std::fmt::{Error, Write};
use std::sync::mpsc::{channel, Receiver};

use crate::elements::{Element, ElementType, Metadata, SimpleElementType};
use crate::threadpools::WRITER_THREAD_POOL;

// wrapper struct that implements std::fmt::Write for any type
// that implements std::io::Write
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
fn push_escaped_char(base: &mut String, input: char) {
    base.push('%');

    // get the UTF-8 code point of the character
    let code_point = input as u32;

    // format the code point as hexadecimal (lowercase)
    let hex = format!("{:x}", code_point);
    base.push_str(&hex);

    base.push('%');
}

// takes a String input, returns an escaped String output
fn push_escaped_string(base: &mut String, input: &str) {
    for c in input.chars() {
        if should_escape_char(c) {
            push_escaped_char(base, c);
        } else {
            base.push(c);
        }
    }
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
        output.push_str(&element.id.to_string());

        if let Some(v) = element.version {
            output.push_str(" v");
            output.push_str(&lexical::to_string(v));
        }

        if let Some(v) = element.visible {
            if v {
                output.push_str(" dV");
            } else {
                output.push_str(" dD");
            }
        }

        if let Some(c) = element.changeset {
            output.push_str(" c");
            output.push_str(&lexical::to_string(c));
        }

        if let Some(t) = element.timestamp {
            output.push_str(" t");
            output.push_str(&t);
        }

        if let Some(u) = element.uid {
            output.push_str(" i");
            output.push_str(&lexical::to_string(u));
        }

        if let Some(u) = element.user {
            output.push_str(" u");
            push_escaped_string(&mut output, &u);
        }

        output.push_str(" T");
        let mut first_tag_written = false;
        for (k, v) in element.tags {
            if first_tag_written {
                output.push(',');
            }
            first_tag_written = true;
            push_escaped_string(&mut output, &k);
            output.push('=');
            push_escaped_string(&mut output, &v);
        }

        match element.element_type {
            ElementType::Node { lat, lon } => {
                output.push_str(" x");
                output.push_str(&lexical::to_string(lon));
                output.push_str(" y");
                output.push_str(&lexical::to_string(lat));
            }
            ElementType::Way { nodes } => {
                output.push_str(" N");
                let mut first_node_written = false;
                for n in nodes {
                    if first_node_written {
                        output.push(',');
                    }
                    first_node_written = true;
                    output.push('n');
                    output.push_str(&lexical::to_string(n));
                }
            }
            ElementType::Relation { members } => {
                output.push_str(" M");
                let mut first_member_written = false;
                for m in members {
                    if first_member_written {
                        output.push(',');
                    }
                    first_member_written = true;
                    output.push(match m.t {
                        Some(SimpleElementType::Node) => 'n',
                        Some(SimpleElementType::Way) => 'w',
                        Some(SimpleElementType::Relation) => 'r',
                        None => panic!("Member type is None"),
                    });
                    output.push_str(&lexical::to_string(m.id));
                    output.push('@');
                    if let Some(role) = m.role {
                        push_escaped_string(&mut output, &role);
                    }
                    // TODO: determine better course of action if role is None?
                }
            }
        }
        output.push('\n');
    }
    Ok(output)
}

#[allow(unused_variables)]
pub fn write_opl<D: std::io::Write>(receiver: Receiver<Vec<Element>>, metadata: Metadata, dest: D) {
    let mut writer = ToFmtWrite(dest);
    let (output_sender, output_reciever) = channel();
    WRITER_THREAD_POOL.install(move || {
        receiver
            .into_iter()
            .par_bridge()
            .map(serialize_chunk)
            .map(|result| result.expect("Failed to serialize chunk"))
            .for_each(|s| match output_sender.clone().send(s) {
                Ok(_) => (),
                Err(e) => panic!("Error passing output chunk between threads."),
            });
    });

    for output_string in output_reciever {
        writer
            .write_str(&output_string)
            .expect("Failed to write to output");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_escaped_char() {
        let mut string1 = String::new();
        push_escaped_char(&mut string1, ' ');
        assert_eq!(string1, "%20%");

        let mut string2 = String::new();
        push_escaped_char(&mut string2, ',');
        assert_eq!(string2, "%2c%");

        let mut string3 = String::new();
        push_escaped_char(&mut string3, '😱');
        assert_eq!(string3, "%1f631%");

        let mut string4 = String::new();
        push_escaped_char(&mut string4, '𒄈');
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
    fn test_push_escaped_string() {
        let mut string1 = String::new();
        push_escaped_string(&mut string1, "A,B");
        assert_eq!(string1, "A%2c%B");

        let mut string2 = String::new();
        push_escaped_string(&mut string2, "ohmy😱goodness");
        assert_eq!(string2, "ohmy%1f631%goodness");
    }
}
