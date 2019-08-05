use std::fs::File;
use std::io::{Read, Write};

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

const BR_OPCODE: u16 = 0x0000;
const ADD_OPCODE: u16 = 0x1000;
const LD_OPCODE: u16 = 0x2000;
const ST_OPCODE: u16 = 0x3000;
const JSR_OPCODE: u16 = 0x4000;
const AND_OPCODE: u16 = 0x5000;
const LDR_OPCODE: u16 = 0x6000;
const STR_OPCODE: u16 = 0x7000;
const RTI_OPCODE: u16 = 0x8000;
const NOT_OPCODE: u16 = 0x9000;
const LDI_OPCODE: u16 = 0xA000;
const STI_OPCODE: u16 = 0xB000;
const JMP_OPCODE: u16 = 0xC000;
const RESERVED_OP: u16 = 0xD000;
const LEA_OPCODE: u16 = 0xE000;
const TRAP_OPCODE: u16 = 0xF000;

pub struct Simulator {
    memory: [u16; 0xFFFF],
    registers: [u16; 8],
    pc: u16,
    ir: u16,
    cc: char,
    input: Reader,
    display: Writer,
    tracer: Tracer,
}

impl Simulator {
    pub fn new(input: Reader, display: Writer, tracer: Tracer) -> Self {
        let mut memory = [0; 0xFFFF];
        memory[CLK] = 0x8000;
        memory[DSR] = 0x8000;
        Self {
            memory,
            registers: [0; 8],
            pc: 0,
            ir: 0,
            cc: 'Z',
            input,
            display,
            tracer,
        }
    }

    pub fn with_operating_system(self, file: &str) -> Self {
        self.load(file).expect("Unable to load file")
    }

    pub fn load(mut self, file: &str) -> Result<Self, String> {
        let mut file = match File::open(file) {
            Err(e) => {
                return Err(format!("{}", e));
            }
            f => f.unwrap(),
        };

        let mut buffer = Vec::new();
        if let Err(e) = file.read_to_end(&mut buffer) {
            return Err(format!("{}", e));
        }

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
            'Z'
        } else if value & 0x8000 == 0 {
            'P'
        } else {
            'N'
        };
    }

    fn fetch(&mut self) {
        self.ir = self.memory[self.pc as usize];
        self.pc = self.pc.wrapping_add(1);
    }

    fn trace(&mut self) {
        if self.tracer.wants((self.ir & 0xF000) >> 12, self.pc) {
            self.tracer.trace(
                format!(
                    "After executing instruction: 0x{:04X}\n{}Program Counter: 0x{:04X}\nCondition Code: {}\n===================================\n",
                    self.ir,
                    (0..8)
                        .map(|i| format!("Register {}: 0x{:04X}\n", i, self.registers[i]))
                        .collect::<String>(),
                    self.pc,
                    self.cc
                )
                .as_ref(),
            );
        }
    }

    pub fn execute(&mut self) {
        while self.read_memory(CLK as u16) & 0x8000 != 0 {
            self.fetch();
            self.step();
            self.trace();
        }
    }

    fn read_memory(&mut self, address: u16) -> u16 {
        match address as usize {
            DDR => 0x0000,
            KBSR => {
                let mut buf = [0; 1];
                match self.input.read(&mut buf) {
                    Ok(x) if x != 0 => {
                        self.memory[KBDR] = u16::from(buf[0]);
                        0x8000
                    }
                    Err(_) => {
                        println!(
                            "\n--- Program requires more input than provided in the input file ---"
                        );
                        std::process::exit(2);
                    }
                    _ => 0x0000,
                }
            }
            KBDR => self.memory[KBDR],
            addr => self.memory[addr],
        }
    }

    pub fn write_memory(&mut self, address: u16, value: u16) {
        match address as usize {
            KBSR | KBDR | DSR => {}
            DDR => {
                self.memory[DDR] = 0;
                self.memory[DSR] = 0x8000;
                let value = value as u8;
                self.display
                    .write(
                        format!("{}{}", if value == 0xA { "\r" } else { "" }, value as char)
                            .as_ref(),
                    )
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

        let destination_register = usize::from((self.ir & 0x0E00) >> 9);
        let source_register_one = usize::from((self.ir & 0x01C0) >> 6);
        let source_register_two = usize::from(self.ir & 0x0007);
        let pc_offset_9 = (((self.ir & 0x01FF) << 7) as i16) >> 7;
        let offset_6 = (((self.ir & 0x003F) << 10) as i16) >> 10;
        let imm5 = (((self.ir & 0x001F) << 11) as i16) >> 11;

        match opcode {
            BR_OPCODE => {
                let n = (self.ir & 0x0800) != 0;
                let z = (self.ir & 0x0400) != 0;
                let p = (self.ir & 0x0200) != 0;

                if (n && self.cc == 'N') || (z && self.cc == 'Z') || (p && self.cc == 'P') {
                    self.pc = (self.pc as i16 + pc_offset_9) as u16;
                }
            }
            ADD_OPCODE => {
                let source_two = if (self.ir & 0x20) == 0 {
                    self.registers[source_register_two] as i16
                } else {
                    imm5
                };

                let result =
                    (self.registers[source_register_one] as i16).wrapping_add(source_two) as u16;

                self.registers[destination_register] = result;
                self.update_cc(result);
            }
            LD_OPCODE => {
                let value = self.read_memory((self.pc as i16 + pc_offset_9) as u16);

                self.registers[destination_register] = value;
                self.update_cc(value);
            }
            ST_OPCODE => {
                let address = (self.pc as i16 + pc_offset_9) as u16;

                self.write_memory(address, self.registers[destination_register]);
            }
            JSR_OPCODE => {
                self.registers[7] = self.pc;

                self.pc = if self.ir & 0x0800 == 0 {
                    self.registers[source_register_one]
                } else {
                    (self.pc as i16 + ((((self.ir & 0x7FF) << 5) as i16) >> 5)) as u16
                };
            }
            AND_OPCODE => {
                let source_two = if (self.ir & 0x20) == 0 {
                    self.registers[source_register_two] as i16
                } else {
                    imm5
                };

                let result = (self.registers[source_register_one] as i16 & source_two) as u16;

                self.registers[destination_register] = result;
                self.update_cc(result);
            }
            LDR_OPCODE => {
                let value = self
                    .read_memory((self.registers[source_register_one] as i16 + offset_6) as u16);

                self.registers[destination_register] = value;
                self.update_cc(value);
            }
            STR_OPCODE => {
                let address = (self.registers[source_register_one] as i16 + offset_6) as u16;

                self.write_memory(address, self.registers[destination_register]);
            }
            NOT_OPCODE => {
                let value = !self.registers[source_register_one];;

                self.registers[destination_register] = value;
                self.update_cc(value);
            }
            LDI_OPCODE => {
                let indirect = self.read_memory((self.pc as i16 + pc_offset_9) as u16);
                let value = self.read_memory(indirect);

                self.registers[destination_register] = value;
                self.update_cc(value);
            }
            STI_OPCODE => {
                let indirect = self.read_memory((self.pc as i16 + pc_offset_9) as u16);

                self.write_memory(indirect, self.registers[destination_register]);
            }
            JMP_OPCODE => {
                self.pc = self.registers[source_register_one];
            }
            LEA_OPCODE => {
                let address = (self.pc as i16 + pc_offset_9) as u16;

                self.registers[destination_register] = address;
                self.update_cc(address);
            }
            TRAP_OPCODE => {
                self.registers[7] = self.pc;

                let trap_vector = (self.ir & 0xFF) as usize;
                self.pc = self.memory[trap_vector];
            }

            RTI_OPCODE | RESERVED_OP => {}
            _ => unreachable!(),
        }
    }
}
