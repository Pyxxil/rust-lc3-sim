use std::fs::File;
use std::io::{BufWriter, Write, Result};

use std::str;

pub enum Writer {
    Terminal(crossterm::Terminal),
    OutFile(BufWriter<File>),
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
