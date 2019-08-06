extern crate clap;
extern crate crossterm;

use clap::{App, Arg};

use lc3simlib::simulator;
use simulator::{Reader, Simulator, Tracer, Writer};

fn valid_instruction(instr: String) -> Result<(), String> {
    match instr.to_ascii_uppercase().as_ref() {
        "BR" | "ADD" | "LD" | "ST" | "JSR" | "JSRR" | "AND" | "LDR" | "STR" | "RTI" | "NOT"
        | "LDI" | "STI" | "JMP" | "LEA" | "TRAP" => Ok(()),
        _ => Err(String::from("Expected a valid instruction")),
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
                .help("Any data files to use")
                .takes_value(true)
                .multiple(true)
                .number_of_values(1),
        )
        .get_matches();

    let simulator = args
        .values_of("data")
        .and_then(|data| Some(data.collect::<Vec<_>>()))
        .unwrap_or_default()
        .iter()
        .fold(
            Simulator::new(
                Reader::from(args.value_of("input")),
                Writer::from(args.value_of("output")),
                Tracer::from((
                    args.value_of("trace"),
                    args.values_of("instr").and_then(|v| Some(v.collect())),
                    args.is_present("user"),
                )),
            )
            .with_operating_system(args.value_of("os").unwrap()),
            |sim, data| match sim.load(data) {
                Ok(simulator) => simulator,
                Err(e) => {
                    println!("Error: {}", e);
                    panic!();
                }
            },
        );

    match simulator.load(args.value_of("file").unwrap()) {
        Ok(simulator) => {
            simulator.execute();
        }
        Err(e) => println!("Error: {}", e),
    };
}
