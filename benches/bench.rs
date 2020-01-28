#![feature(test)]

#[macro_use]
extern crate criterion;
use criterion::Criterion;

extern crate crossterm;

extern crate lc3simlib;
use lc3simlib::simulator::{Reader, Simulator, Tracer, Writer};

use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter};

fn simulate(file: &str) {
    let simulator = Simulator::new(
        Reader::InFile(BufReader::new(
            OpenOptions::new().read(true).open("test.in").unwrap(),
        )),
        Writer::OutFile(BufWriter::new(
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("test.out")
                .unwrap(),
        )),
        Tracer::TraceFile(
            BufWriter::new(
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open("trace.out")
                    .unwrap(),
            ),
            0xFFFF,
            false,
        ),
    )
    .with_operating_system("LC3_OS.obj")
    .load(file);

    match simulator {
        Ok(sim) => {
            sim.run();
        }
        _ => {}
    }
}

fn bench_simulator(c: &mut Criterion) {
    c.bench_function("simulate Input", |b| b.iter(|| simulate("input.obj")));
    c.bench_function("simulate Fibonacci", |b| {
        b.iter(|| simulate("Fibonacci.obj"))
    });
    c.bench_function("simulate Recursive Fibonacci", |b| {
        b.iter(|| simulate("Recursive_Fibonacci.obj"))
    });
}

criterion_group!(benches, bench_simulator);
criterion_main!(benches);
