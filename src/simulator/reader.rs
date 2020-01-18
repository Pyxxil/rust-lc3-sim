use std::convert::From;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Error, ErrorKind, Read};

use crossterm::{input, InputEvent, KeyEvent, RawScreen, SyncReader};

/// An enum used to determine where to take input to the program from
pub enum Reader {
    Keyboard(Result<RawScreen, Error>, SyncReader),
    InFile(BufReader<File>),
}

impl From<Option<&str>> for Reader {
    fn from(file: Option<&str>) -> Self {
        file.map(|f| {
            Self::InFile(BufReader::new(
                OpenOptions::new().read(true).open(f).unwrap(),
            ))
        })
        .unwrap_or_default()
    }
}

impl Default for Reader {
    fn default() -> Self {
        Self::Keyboard(RawScreen::into_raw_mode(), input().read_sync())
    }
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
                match reader.next() {
                    Some(InputEvent::Keyboard(KeyEvent::Char(key))) => {
                        buf[0] = key as u8;
                        Ok(1)
                    }
                    Some(InputEvent::Keyboard(KeyEvent::Left)) => {
                        buf[0] = b'a';
                        Ok(1)
                    }
                    Some(InputEvent::Keyboard(KeyEvent::Up)) => {
                        buf[0] = b'w';
                        Ok(1)
                    }
                    Some(InputEvent::Keyboard(KeyEvent::Down)) => {
                        buf[0] = b's';
                        Ok(1)
                    }
                    Some(InputEvent::Keyboard(KeyEvent::Right)) => {
                        buf[0] = b'd';
                        Ok(1)
                    }
                    Some(InputEvent::Keyboard(KeyEvent::Esc)) => {
                        // If the user hits the ESC key, then we want to exit. Of course, this only works if the program asks for input.
                        Err(Error::new(ErrorKind::Interrupted, ""))
                    }
                    _ => {
                        // Basically, if this is hit nothing bad has happened, so let's just return Ok anyways (however, indicate that nothing was read)
                        Ok(0)
                    }
                }
            }
            // Input from a file is just gathered from that file. We only read a single byte here (or, at least, buf should only have len 1)
            Reader::InFile(ref mut file) => {
                match file.read(buf) {
                    Ok(x) if x > 0 => Ok(x),
                    // Essentially if we reach a problem (likely EOF) we just want to return an error and exit early.
                    // This should, at the very least, let the user know that their program has required more input than was available in the file.
                    _ => Err(Error::new(ErrorKind::NotFound, "")),
                }
            }
        }
    }
}
