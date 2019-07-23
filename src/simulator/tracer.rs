use std::fs::File;
use std::io::{BufWriter, Write};

pub enum Tracer {
    NoTrace,
    TraceFile(BufWriter<File>, u16, bool),
}

/// A trait meant for implementing the tracing ability of a tracer
pub trait Trace {
    /// Whether or not the tracer wants to trace the instruction
    fn wants(&self, instruction: u16, pc: u16) -> bool;
    /// The specific implementation of the trace
    fn trace(&mut self, string: &str);
}

impl Trace for Tracer {
    fn wants(&self, instruction: u16, pc: u16) -> bool {
        match self {
            Tracer::NoTrace => false,
            Tracer::TraceFile(_, want, userspace) => {
                (!userspace || pc >= 0x3000) && (want & (1 << instruction)) != 0
            }
        }
    }

    fn trace(&mut self, string: &str) {
        match self {
            Tracer::NoTrace => {}
            Tracer::TraceFile(ref mut file, _, _) => match write!(file, "{}", string) {
                _ => {}
            },
        }
    }
}
