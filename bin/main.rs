extern crate clap;
extern crate crossterm;

use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter};

use clap::{App, Arg, Values};
use crossterm::{input, terminal, RawScreen};

use lc3simlib::simulator;
use simulator::{Reader, Simulator, Tracer, Writer};

fn valid_instruction(instr: String) -> Result<(), String> {
    match instr.to_ascii_uppercase().as_ref() {
        "BR" | "ADD" | "LD" | "ST" | "JSR" | "JSRR" | "AND" | "LDR" | "STR" | "RTI" | "NOT"
        | "LDI" | "STI" | "JMP" | "LEA" | "TRAP" => Ok(()),
        _ => Err(String::from("Expected a valid instruction")),
    }
}

fn get_tracer(file: Option<&str>, instructions: Option<Values>, user_only: bool) -> Tracer {
    if let Some(f) = file {
        let trace_instructions = if let Some(instrs) = instructions {
            instrs.fold(0, |acc, instr| {
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

        Tracer::TraceFile(
            BufWriter::new(
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(f)
                    .unwrap(),
            ),
            trace_instructions,
            user_only,
        )
    } else {
        Tracer::NoTrace
    }
}

fn get_output_device(file: Option<&str>) -> Writer {
    if let Some(f) = file {
        Writer::OutFile(BufWriter::new(
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(f)
                .unwrap(),
        ))
    } else {
        Writer::Terminal(terminal())
    }
}

fn get_input_device(file: Option<&str>) -> Reader {
    if let Some(f) = file {
        Reader::InFile(BufReader::new(
            OpenOptions::new().read(true).open(f).unwrap(),
        ))
    } else {
        Reader::Keyboard(RawScreen::into_raw_mode(), input().read_async())
    }
}

fn main() {
    let args = App::new("lc3sim")
        .arg(Arg::with_name("file").required(true))
        .arg(
            Arg::with_name("output")
                .long("output")
                .short("o")
                .help("The output file (for writing to)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("input")
                .long("input")
                .short("i")
                .help("The input file (for reading from)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("trace")
                .long("trace")
                .short("t")
                .help("The trace file to use")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("instr")
                .long("instr")
                .help("Trace a specific instruction")
                .takes_value(true)
                .multiple(true)
                .number_of_values(1)
                .validator(valid_instruction),
        )
        .arg(
            Arg::with_name("user")
                .long("user-only")
                .short("u")
                .help("Only trace user space instructions (instructions at addresses >= 0x3000)"),
        )
        .arg(
            Arg::with_name("os")
                .long("os")
                .help("The operating system to use")
                .takes_value(true)
                .default_value("./LC3_OS.obj"),
        )
        .arg(
            Arg::with_name("data")
                .long("data")
                .short("d")
                .takes_value(true)
                .help("Any data files to use")
                .multiple(true)
                .number_of_values(1),
        )
        .get_matches();

    let simulator = Simulator::new(
        get_input_device(args.value_of("input")),
        get_output_device(args.value_of("output")),
        get_tracer(
            args.value_of("trace"),
            args.values_of("instr"),
            args.is_present("user"),
        ),
    )
    .with_operating_system(args.value_of("os").unwrap());

    let simulator = if let Some(data) = args.values_of("data") {
        data.fold(simulator, |sim, data| match sim.load(data) {
            Ok(simulator) => simulator,
            Err(e) => {
                println!("Error: {}", e);
                panic!();
            }
        })
    } else {
        simulator
    };

    match simulator.load(args.value_of("file").unwrap()) {
        Ok(mut simulator) => {
            simulator.execute();
        }
        Err(e) => println!("Error: {}", e),
    };
}