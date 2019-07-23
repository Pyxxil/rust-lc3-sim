use std::io::{BufReader, Error, ErrorKind, Read};
use std::fs::File;

use crossterm::{AsyncReader, InputEvent, KeyEvent, RawScreen};

pub enum Reader {
    Keyboard(Result<RawScreen, Error>, AsyncReader),
    InFile(BufReader<File>),
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        match self {
            Reader::Keyboard(_, ref mut reader) => {
                if let Some(InputEvent::Keyboard(KeyEvent::Char(key))) = reader.next() {
                    buf[0] = key as u8;
                    Ok(1)
                } else {
                    Ok(0)
                }
            }
            Reader::InFile(ref mut file) => {
                return match file.read_exact(buf) {
                    Ok(_) => Ok(1),
                    _ => Err(Error::new(ErrorKind::NotFound, "")),
                };
            }
        }
    }
}
