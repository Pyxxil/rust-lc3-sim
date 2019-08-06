use std::convert::From;
use std::default::Default;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Result, Write};

use std::str;

use crossterm::terminal;

pub enum Writer {
    Terminal(crossterm::Terminal),
    OutFile(BufWriter<File>),
}

impl From<Option<&str>> for Writer {
    fn from(file: Option<&str>) -> Self {
        file.and_then(|f| {
            Some(Self::OutFile(BufWriter::new(
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(f)
                    .unwrap(),
            )))
        })
        .unwrap_or_default()
    }
}

impl Default for Writer {
    fn default() -> Self {
        Self::Terminal(terminal())
    }
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let s = str::from_utf8(&buf).unwrap();
        match self {
            Writer::Terminal(ref mut terminal) => match terminal.write(s) {
                _ => {}
            },
            Writer::OutFile(ref mut file) => match write!(file, "{}", s) {
                _ => {}
            },
        }

        Ok(s.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
