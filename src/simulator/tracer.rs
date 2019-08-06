use std::convert::From;
use std::default::Default;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};

pub enum Tracer {
    NoTrace,
    TraceFile(BufWriter<File>, u16, bool),
}

impl From<(Option<&str>, Option<Vec<&str>>, bool)> for Tracer {
    fn from(args: (Option<&str>, Option<Vec<&str>>, bool)) -> Self {
        args.0
            .and_then(|f| {
                let trace_instructions = if let Some(instrs) = args.1 {
                    instrs.iter().fold(0, |acc, instr| {
                        acc | match instr.to_ascii_uppercase().as_ref() {
                            "BR" => 0x1,
                            "ADD" => 0x2,
                            "LD" => 0x4,
                            "ST" => 0x8,
                            "JSR" | "JSRR" => 0x10,
                            "AND" => 0x20,
                            "LDR" => 0x40,
                            "STR" => 0x80,
                            "RTI" => 0x100,
                            "NOT" => 0x200,
                            "LDI" => 0x400,
                            "STI" => 0x800,
                            "JMP" => 0x1000,
                            "LEA" => 0x4000,
                            "TRAP" => 0x8000,
                            _ => unreachable!(),
                        }
                    })
                } else {
                    0xFFFF
                };

                Some(Self::TraceFile(
                    BufWriter::new(
                        OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .create(true)
                            .open(f)
                            .unwrap(),
                    ),
                    trace_instructions,
                    args.2,
                ))
            })
            .unwrap_or_default()
    }
}

impl Default for Tracer {
    fn default() -> Self {
        Self::NoTrace
    }
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
