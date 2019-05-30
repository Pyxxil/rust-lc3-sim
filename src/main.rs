extern crate crossterm;
use crossterm::RawScreen;

mod simulator;
use simulator::Simulator;

fn main() {
    let _screen = RawScreen::into_raw_mode();

    let mut simulator = Simulator::new().with_operating_system("../LC3_OS.obj");
    match simulator.load("../Fibonacci.obj") {
        Ok(()) => simulator.execute(),
        Err(e) => println!("Error: {}", e),
    };
}
