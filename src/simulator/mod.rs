use std::fs::File;
use std::io::{Error, ErrorKind, Read, Write};

pub mod reader;
pub mod tracer;
pub mod writer;

pub use reader::Reader;
pub use tracer::{Trace, Tracer};
pub use writer::Writer;

const CLK: usize = 0xFFFE;
const KBSR: usize = 0xFE00;
const KBDR: usize = 0xFE02;
const DSR: usize = 0xFE04;
const DDR: usize = 0xFE06;

const OPCODE_BR: u16 = 0x0000;
const OPCODE_ADD: u16 = 0x1000;
const OPCODE_LD: u16 = 0x2000;
const OPCODE_ST: u16 = 0x3000;
const OPCODE_JSR: u16 = 0x4000;
const OPCODE_AND: u16 = 0x5000;
const OPCODE_LDR: u16 = 0x6000;
const OPCODE_STR: u16 = 0x7000;
const OPCODE_RTI: u16 = 0x8000;
const OPCODE_NOT: u16 = 0x9000;
const OPCODE_LDI: u16 = 0xA000;
const OPCODE_STI: u16 = 0xB000;
const OPCODE_JMP: u16 = 0xC000;
const RESERVED: u16 = 0xD000;
const OPCODE_LEA: u16 = 0xE000;
const OPCODE_TRAP: u16 = 0xF000;

const fn sign_extend(val: u16, length: u16) -> i16 {
    (val << (16 - length)) as i16 >> (16 - length)
}

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
        let mut memory = [0; 0xFFFF];
        memory[CLK] = 0x8000;
        memory[DSR] = 0x8000;
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
            self.memory[address as usize] = u16::from(buffer[i]) << 8 | u16::from(buffer[i + 1]);
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
        self.ir = self.memory[self.pc as usize];
        self.pc = self.pc.wrapping_add(1);
    }

    fn trace(&mut self) {
        if self.tracer.wants(self.ir >> 12 & 0b1111, self.pc) {
            self.tracer.trace(
                format!(
                    "After executing instruction: 0x{:04X}\n{}Program Counter: 0x{:04X}\nCondition Code: {}\n===================================\n",
                    self.ir,
                    (0..8)
                        .map(|i| format!("Register {}: 0x{:04X}\n", i, self.registers[i]))
                        .collect::<String>(),
                    self.pc,
                    if self.cc & 0b100 != 0 { 'N' } else if self.cc & 0b010 == 0 { 'P' } else { 'Z' }
                )
                .as_ref(),
            );
        }
    }

    pub fn execute(mut self) {
        while self.read(CLK as u16) & 0x8000 != 0 {
            self.fetch();
            self.step();
            self.trace();
        }
    }

    fn read(&mut self, address: u16) -> u16 {
        match address as usize {
            DDR => 0x0000,
            KBSR => {
                let mut buf = [0; 1];
                match self.input.read(&mut buf) {
                    Ok(x) if x != 0 => {
                        self.memory[KBDR] = u16::from(buf[0]);
                        0x8000
                    }
                    Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                        println!("\r\n--- ESC pressed. Quitting simulator ---\r");
                        self.memory[CLK] = 0x0000;
                        0x0000
                    }
                    Err(_) => {
                        println!(
                            "\r\n--- Program requires more input than provided in the input file ---\r"
                        );
                        self.memory[CLK] = 0x0000;
                        0x0000
                    }
                    _ => 0x0000,
                }
            }
            addr => self.memory[addr],
        }
    }

    pub fn write(&mut self, address: u16, value: u16) {
        match address as usize {
            DDR => {
                self.memory[DDR] = 0x0000;
                self.memory[DSR] = 0x8000;
                let value = value as u8 as char;
                let _ = self
                    .display
                    .write(format!("{}{}", if value == '\n' { "\r" } else { "" }, value).as_ref())
                    .unwrap_or_else(|_| {
                        self.memory[DSR] = 0;
                        0
                    });
            }
            addr => {
                self.memory[addr] = value;
            }
        }
    }

    fn step(&mut self) {
        let opcode = self.ir & 0xF000;

        let destination_register = usize::from(self.ir >> 9 & 0b111);
        let source_register_one = usize::from(self.ir >> 6 & 0b111);
        let source_register_two = usize::from(self.ir & 0b111);
        let pc_offset_9 = sign_extend(self.ir, 9);
        let offset_6 = sign_extend(self.ir, 6);
        let imm5 = sign_extend(self.ir, 5);

        match opcode {
            OPCODE_BR => {
                if destination_register & self.cc != 0 {
                    self.pc = (self.pc as i16 + pc_offset_9) as u16;
                }
            }
            OPCODE_ADD => {
                let source_two = if self.ir & 0x20 == 0 {
                    self.registers[source_register_two] as i16
                } else {
                    imm5
                };

                let result =
                    (self.registers[source_register_one] as i16).wrapping_add(source_two) as u16;

                self.registers[destination_register] = result;
                self.update_cc(result);
            }
            OPCODE_LD => {
                let value = self.read((self.pc as i16 + pc_offset_9) as u16);

                self.registers[destination_register] = value;
                self.update_cc(value);
            }
            OPCODE_ST => {
                let address = (self.pc as i16 + pc_offset_9) as u16;

                self.write(address, self.registers[destination_register]);
            }
            OPCODE_JSR => {
                self.registers[7] = self.pc;

                self.pc = if self.ir & 0x0800 == 0 {
                    self.registers[source_register_one]
                } else {
                    (self.pc as i16 + sign_extend(self.ir, 11)) as u16
                };
            }
            OPCODE_AND => {
                let source_two = if self.ir & 0x20 == 0 {
                    self.registers[source_register_two] as i16
                } else {
                    imm5
                };

                let result = (self.registers[source_register_one] as i16 & source_two) as u16;

                self.registers[destination_register] = result;
                self.update_cc(result);
            }
            OPCODE_LDR => {
                let value =
                    self.read((self.registers[source_register_one] as i16 + offset_6) as u16);

                self.registers[destination_register] = value;
                self.update_cc(value);
            }
            OPCODE_STR => {
                let address = (self.registers[source_register_one] as i16 + offset_6) as u16;

                self.write(address, self.registers[destination_register]);
            }
            OPCODE_NOT => {
                let value = !self.registers[source_register_one];

                self.registers[destination_register] = value;
                self.update_cc(value);
            }
            OPCODE_LDI => {
                let indirect = self.read((self.pc as i16 + pc_offset_9) as u16);
                let value = self.read(indirect);

                self.registers[destination_register] = value;
                self.update_cc(value);
            }
            OPCODE_STI => {
                let indirect = self.read((self.pc as i16 + pc_offset_9) as u16);

                self.write(indirect, self.registers[destination_register]);
            }
            OPCODE_JMP => {
                self.pc = self.registers[source_register_one];
            }
            OPCODE_LEA => {
                let address = (self.pc as i16 + pc_offset_9) as u16;

                self.registers[destination_register] = address;
                self.update_cc(address);
            }
            OPCODE_TRAP => {
                self.registers[7] = self.pc;

                let trap_vector = (self.ir & 0xFF) as usize;
                self.pc = self.memory[trap_vector];
            }

            OPCODE_RTI | RESERVED => {}
            _ => unreachable!(),
        }
    }
}
