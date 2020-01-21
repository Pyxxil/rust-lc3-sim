use std::fs::File;
use std::io::{Error, ErrorKind, Read, Write};

mod instruction;
pub mod reader;
pub mod tracer;
pub mod writer;

pub use reader::Reader;
pub use tracer::{Trace, Tracer};
pub use writer::Writer;

use instruction::*;

const CLK: u16 = 0xFFFE;
const KBSR: u16 = 0xFE00;
const KBDR: u16 = 0xFE02;
const DSR: u16 = 0xFE04;
const DDR: u16 = 0xFE06;

pub struct Simulator {
    memory: [u16; 0xFFFF],
    registers: [u16; 8],
    pc: u16,
    ir: u16,
    cc: usize,
    input: Reader,
    display: Writer,
    tracer: Tracer,
}

impl Simulator {
    #[must_use]
    pub fn new(input: Reader, display: Writer, tracer: Tracer) -> Self {
        let mut memory = [0x0000; 0xFFFF];
        memory[CLK as usize] = 0x8000;
        memory[DSR as usize] = 0x8000;
        Self {
            memory,
            registers: [0; 8],
            pc: 0,
            ir: 0,
            cc: 0b010,
            input,
            display,
            tracer,
        }
    }

    #[must_use]
    pub fn with_operating_system(self, file: &str) -> Self {
        self.load(file).expect("Unable to load Operating System")
    }

    /// Load the specified file into the simulator.
    ///
    /// # Errors
    /// Will return Err if the supplied file was unable to be read from
    pub fn load(mut self, file: &str) -> Result<Self, Error> {
        let mut file = File::open(file)?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let mut address = u16::from(buffer[0]) << 8 | u16::from(buffer[1]);

        self.pc = address;

        (2..buffer.len()).step_by(2).for_each(|i| {
            self.write_memory(
                address,
                u16::from(buffer[i]) << 8 | u16::from(buffer[i + 1]),
            );

            address += 1;
        });

        Ok(self)
    }

    fn update_cc(&mut self, value: u16) {
        self.cc = if value == 0 {
            0b010
        } else if value & 0x8000 == 0 {
            0b001
        } else {
            0b100
        };
    }

    fn fetch(&mut self) {
        self.ir = self.read_memory(self.pc);
        self.pc = self.pc.wrapping_add(1);
    }

    fn decode(&self) -> Instruction {
        Instruction::from(self.ir)
    }

    fn execute(&mut self, instruction: Instruction) {
        instruction.execute(self);
    }

    fn trace(&mut self) {
        self.tracer.trace(
                format!(
                    "After executing instruction: 0x{:04X}\n{}Program Counter: 0x{:04X}\nCondition Code: {} {:#?}\n===================================\n",
                    self.ir,
                    (0..8)
                        .map(|i| format!("Register {}: 0x{:04X}\n", i, self.registers[i]))
                        .collect::<String>(),
                    self.pc,
                    if self.cc & 0b100 != 0 { 'N' } else if self.cc & 0b010 == 0 { 'P' } else { 'Z' },
                    Instruction::from(self.ir),
                )
                .as_ref(),
            );
    }

    /// Run the simulator until a problem occurs, or the clock is stopped.
    ///
    /// This attempts to emulate a classic RISC pipeline in nature (with the hopes that in future that will happen).
    /// First, we fetch the next instruction from meory. We then decode this instruction, and execute it.
    ///
    /// We also trace the current instruction if the user wants us to.
    ///
    pub fn run(mut self) {
        while self.read_memory(CLK as u16) & 0x8000 != 0 {
            self.fetch();
            let instruction = self.decode();
            self.execute(instruction);
            if self.tracer.wants(self.ir >> 12 & 0b1111, self.pc) {
                self.trace();
            }
        }
    }

    fn read_register(&self, register: usize) -> u16 {
        self.registers[register]
    }

    /// Read from the specified address, returning the associated contents.
    ///
    /// Some special address are treated differently.
    ///   1. Display Data Register (DDR) [0xFE06]:
    ///     If the read is for this memory-mapped register, we always return 0. Any code that deals
    ///     with the DDR will be checking for a value of 0 to determine that the display is ready to
    ///     be written to. As we control that, we always return 0.
    ///   2. Keyboard Status Register (KBSR) [0xFE00]
    ///     If the read is for this memory-mapped register, we need to check if any input has been
    ///     provided (which can be from the keyboard, or from a file depending on the users choice).
    ///     If there is input, then we place the input into the Display Data Register, and then return
    ///     0x8000 (negative value, as any code attempting to check if input exists will busy-wait for
    ///     this register to become negative).
    ///     If there is no more input (generally this means its from the end of the file used as input),
    ///     then we halt the machine and return 0. If anything else happens then we simply return 0.
    ///
    fn read_memory(&mut self, address: u16) -> u16 {
        match address {
            DDR => 0x0000,
            KBSR => {
                let mut buf = [0; 1];
                match self.input.read(&mut buf) {
                    Ok(x) if x != 0 => {
                        self.write_memory(KBDR, u16::from(buf[0]));
                        0x8000
                    }
                    Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                        println!("\r\n--- ESC pressed. Quitting simulator ---\r");
                        self.write_memory(CLK, 0x0000);
                        0x0000
                    }
                    Err(_) => {
                        println!(
                            "\r\n--- Program requires more input than provided in the input file ---\r"
                        );
                        self.write_memory(CLK, 0x0000);
                        0x0000
                    }
                    _ => 0x0000,
                }
            }
            addr => self.memory[addr as usize],
        }
    }

    pub fn write_register(&mut self, register: usize, value: u16) {
        self.registers[register] = value;
        self.update_cc(value);
    }

    pub fn write_memory(&mut self, address: u16, value: u16) {
        match address {
            DDR => {
                self.memory[DDR as usize] = 0x0000;
                self.memory[DSR as usize] = 0x8000;
                let value = value as u8 as char;
                let _ = self
                    .display
                    .write(format!("{}{}", if value == '\n' { "\r" } else { "" }, value).as_ref())
                    .unwrap_or_else(|_| {
                        self.memory[DSR as usize] = 0x0000;
                        0
                    });
            }
            addr => {
                self.memory[addr as usize] = value;
            }
        }
    }
}
