use std::fmt::{Error, Write};
use std::sync::mpsc::Receiver;

use crate::elements::{Element, ElementType, Metadata};

struct ToFmtWrite<T>(pub T);

impl<T> Write for ToFmtWrite<T>
where
    T: std::io::Write,
{
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|_| std::fmt::Error)
    }
}

fn _should_escape_char(input: char) -> bool {
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

fn _escape_str(input: String) -> String {
    let mut output = String::new();
    for c in input.chars() {
        if _should_escape_char(c) {
            output.push('%');
            output = output + (c as u32).to_string().as_str();
            output.push('%');
        } else {
            // FIXME: this does not appear to be the correct way to escape a character
            // more research needed
            output.push(c);
        }
    }
    output
}

// TODO: encode strings correctly
// see here: https://github.com/osmcode/libosmium/blob/f88048769c13210ca81efca17668dc57ea64c632/include/osmium/io/detail/string_util.hpp#L204-L237

fn _write_elements<W: Write>(receiver: Receiver<Element>, mut w: W) -> Result<(), Error> {
    for element in receiver {
        match element.element_type {
            ElementType::Node { .. } => {
                w.write_char('n')?;
            }
            ElementType::Way { .. } => {
                w.write_char('w')?;
            }
            ElementType::Relation { .. } => {
                w.write_char('r')?;
            }
        }
        let id = &element.id;
        write!(w, "{id}")?;

        if let Some(v) = element.version {
            write!(w, " v{v}")?;
        }

        if let Some(v) = element.visible {
            if v {
                write!(w, " dV")?;
            } else {
                write!(w, "dD")?;
            }
        }

        if let Some(c) = element.changeset {
            write!(w, " c{c}")?;
        }

        if let Some(t) = element.timestamp {
            write!(w, " t{t}")?;
        }

        if let Some(u) = element.uid {
            write!(w, " i{u}")?;
        }

        if let Some(u) = element.user {
            let escaped_username = _escape_str(u);
            write!(w, " u{escaped_username}")?;
        }

        write!(w, " T")?;
        let tags_out = element
            .tags
            .into_iter()
            .map(|(k, v)| (_escape_str(k), _escape_str(v)))
            .map(|(k, v)| format!("{k}={v}"))
            .reduce(|acc, s| format!("{acc},{s}"))
            .unwrap_or_default();
        write!(w, "{tags_out}")?;

        match element.element_type {
            ElementType::Node { lat, lon } => {
                write!(w, " x{lon} y{lat}")?;
            }
            ElementType::Way { nodes } => {
                write!(w, " N")?;
                let out = nodes
                    .into_iter()
                    .map(|n| format!("n{}", n.to_string()))
                    .reduce(|acc, s| format!("{acc},{s}"))
                    .unwrap();
                write!(w, "{out}")?;
            }
            ElementType::Relation { members } => {
                write!(w, " M")?;

                let out = members
                    .into_iter()
                    .map(|m| {
                        let mref = m.id;
                        let mrole = m.role;
                        if let Some(mrole) = mrole {
                            let escaped_member_role = _escape_str(mrole);
                            format!("n{mref}@{escaped_member_role}") // FIXME: assumes node!!!
                        } else {
                            // TODO: Determine role by finding the relevant element?
                            format!("{mref}")
                        }
                    })
                    .reduce(|acc, s| format!("{acc},{s}"))
                    .unwrap();
                write!(w, "{out}")?;
            }
        }
        w.write_char('\n')?;
    }
    Ok(())
}

#[allow(unused_variables)]
pub fn write_opl<D: std::io::Write>(receiver: Receiver<Element>, metadata: Metadata, dest: D) {
    eprintln!("Writing OPL output...");
    let writer = ToFmtWrite(dest);
    match _write_elements(receiver, writer) {
        Ok(_) => (),
        Err(e) => {
            panic!("Error writing output: {e:?}");
        }
    };
}
