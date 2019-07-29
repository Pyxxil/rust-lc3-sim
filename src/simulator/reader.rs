use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Read};

use crossterm::{AsyncReader, InputEvent, KeyEvent, RawScreen};

/// An enum used to determine where to take input to the program from
pub enum Reader {
    Keyboard(Result<RawScreen, Error>, AsyncReader),
    InFile(BufReader<File>),
}

/// Each Reader must implement a form of read.
///
///
///
impl Read for Reader {
    /// # Examples
    /// ```
    /// use lc3simlib::simulator::reader::Reader;
    /// use std::io::{BufReader, Read};
    /// use std::fs::File;
    /// // $ cat test.in
    /// // 23
    /// let mut reader = Reader::InFile(BufReader::new(File::open("test.in").unwrap()));
    /// let mut buf = [0; 1];
    /// assert!(!reader.read(&mut buf).is_err());
    /// assert_eq!(buf, [0x32; 1]);
    /// assert!(!reader.read(&mut buf).is_err());
    /// assert_eq!(buf, [0x33; 1]);
    /// assert!(reader.read(&mut buf).is_err());
    /// ```
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        match self {
            // Input from the keyboard is gathered using crossterm
            Reader::Keyboard(_, ref mut reader) => {
                if let Some(InputEvent::Keyboard(KeyEvent::Char(key))) = reader.next() {
                    buf[0] = key as u8;
                    Ok(1)
                } else {
                    // Basically, if this is hit nothing bad has happened, so let's just return Ok anyways (however, indicate that nothing was read)
                    Ok(0)
                }
            }
            // Input from a file is just gathered from that file. We only read a single byte here (or, at least, buf should only have len 1)
            Reader::InFile(ref mut file) => {
                return match file.read(buf) {
                    Ok(x) if x > 0 => Ok(x),
                    // Essentially if we reach a problem (likely EOF) we just want to return an error and exit early.
                    // This should, at the very least, let the user know that their program has required more input than was available in the file.
                    _ => Err(Error::new(ErrorKind::NotFound, "")),
                };
            }
        }
    }
}
