use crossterm::{input, InputEvent, KeyEvent, AsyncReader, terminal, Terminal};
use std::fs::File;
use std::io;
use std::io::{stdout, Read, Stdout, Write};

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
    program_counter: u16,
    instruction_register: u16,
    condition_code: char,
    input: AsyncReader,
    display: Terminal,
}

impl Simulator {
    pub fn new() -> Self {
        let mut memory = [0; 0xFFFF];
        memory[CLK] = 0x8000;
        memory[DSR] = 0x8000;
        Self {
            memory,
            registers: [0; 8],
            program_counter: 0,
            instruction_register: 0,
            condition_code: 'Z',
            input: input().read_async(),
            display: terminal(),
        }
    }

    pub fn with_operating_system(mut self, file: &str) -> Self {
        self.load(file).unwrap_or_else(|e| println!("Error: {}", e));
        self
    }

    pub fn load(&mut self, file: &str) -> io::Result<()> {
        let mut file = File::open(file)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let mut addr = u16::from(buffer[0]) << 8 | u16::from(buffer[1]);

        self.program_counter = addr;

        (2..buffer.len()).step_by(2).for_each(|i| {
            self.memory[addr as usize] = u16::from(buffer[i]) << 8 | u16::from(buffer[i + 1]);
            addr += 1;
        });

        Ok(())
    }

    fn update_condition_code(&mut self, value: u16) {
        self.condition_code = if value == 0 {
            'Z'
        } else if value & 0x8000 == 0 {
            'P'
        } else {
            'N'
        };
        //write!(self.display, "CONDITION CODE = {}, VALUE = {:04X}\r\n", self.condition_code, value);
    }

    fn fetch(&mut self) {
        self.instruction_register = self.memory[self.program_counter as usize];
        self.program_counter = self.program_counter.wrapping_add(1);
    }

    fn process_interrupts(&mut self) {
        if let Some(key) = self.input.next() {
            if let InputEvent::Keyboard(k) = key {
                if let KeyEvent::Char(c) = k {
                    self.memory[KBDR] = c as u16;
                    self.memory[KBSR] = 0x8000;
                }
            }
        }

        if self.memory[DDR] != 0 {
            self.display.write(
                format!("{}{}",
                    if self.memory[DDR] & 0xFF == 0xA {
                        "\r"
                    } else {
                        ""
                    },
                    (self.memory[DDR] & 0xFF) as u8 as char
            ));
            self.memory[DDR] = 0;
            self.memory[DSR] = 0x8000;
        }
    }

    pub fn execute(&mut self) {
        loop {
            if self.memory[CLK] & 0x8000 == 0 {
                break;
            }

            self.process_interrupts();
            self.fetch();
            //if self.program_counter >= 0x3065 {
            //    write!(
            //        self.display,
            //        "Instruction register {:04X}\r\n",
            //        self.instruction_register
            //    );
            //    write!(
            //        self.display,
            //        "Program counter {:04X}\r\n",
            //        self.program_counter
            //    );
            //}
            self.step();
            //if self.program_counter >= 0x3065 {
            //    for i in 0..8 {
            //        write!(self.display, "Register {} -> {:04X}\r\n", i, self.registers[i]);
            //    }
            //}
        }
    }

    fn step(&mut self) {
        let opcode = self.instruction_register & 0xF000;

        match opcode {
            BR_OPCODE => {
                //writeln!(self.display, "FOUND A BR INSTRUCTION");
                let n = self.instruction_register & 0x0800 != 0;
                let z = self.instruction_register & 0x0400 != 0;
                let p = self.instruction_register & 0x0200 != 0;
                let offset = (((self.instruction_register & 0x1FF) << 7) as i16) >> 7;

                if (n && self.condition_code == 'N')
                    || (z && self.condition_code == 'Z')
                    || (p && self.condition_code == 'P')
                {
                    self.program_counter = (self.program_counter as i16 + offset) as u16;
                }
            }
            ADD_OPCODE => {
                //writeln!(self.display, "FOUND AN ADD INSTRUCTION");
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                //write!(self.display, "DESTINATION REGISTER => {}\r\n", destination_register);
                let source_one =
                    self.registers[((self.instruction_register & 0x01C0) >> 6) as usize] as i16;
                let source_two = if (self.instruction_register & 0x20) == 0 {
                    self.registers[(self.instruction_register & 0x0007) as usize] as i16
                } else {
                    (((self.instruction_register & 0x1F) << 11) as i16) >> 11
                };

                let result = source_one.wrapping_add(source_two) as u16;

                self.registers[destination_register as usize] = result;
                self.update_condition_code(result);
            }
            LD_OPCODE => {
                //writeln!(self.display, "FOUND AN LD INSTRUCTION");
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let offset = (((self.instruction_register & 0x1FF) << 7) as i16) >> 7;
                let address = (self.program_counter as i16 + offset) as usize;

                if address == KBDR {
                    self.memory[KBSR] = 0;
                }

                self.registers[destination_register as usize] =
                    self.memory[(self.program_counter as i16 + offset) as usize];
                self.update_condition_code(self.registers[destination_register as usize]);
            }
            ST_OPCODE => {
                //writeln!(self.display, "FOUND AN ST INSTRUCTION");
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let offset = (((self.instruction_register & 0x1FF) << 7) as i16) >> 7;

                self.memory[(self.program_counter as i16 + offset) as usize] =
                    self.registers[destination_register as usize];
            }
            JSR_OPCODE => {
                //writeln!(self.display, "FOUND A JSR INSTRUCTION");
                self.registers[7] = self.program_counter;

                if self.instruction_register & 0x0800 == 0 {
                    self.program_counter =
                        self.registers[((self.instruction_register & 0x1C0) >> 6) as usize];
                } else {
                    let offset = (((self.instruction_register & 0x1FF) << 7) as i16) >> 7;
                    self.program_counter = (self.program_counter as i16 + offset) as u16;
                }
            }
            AND_OPCODE => {
                //writeln!(self.display, "FOUND AN AND INSTRUCTION");
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let source_one =
                    self.registers[((self.instruction_register & 0x01C0) >> 6) as usize] as i16;
                let source_two = if (self.instruction_register & 0x20) == 0 {
                    self.registers[(self.instruction_register & 0x0007) as usize] as i16
                } else {
                    (((self.instruction_register & 0x1F) << 11) as i16) >> 11
                };

                let result = (source_one & source_two) as u16;

                self.registers[destination_register as usize] = result;
                self.update_condition_code(result);
            }
            LDR_OPCODE => {
                //writeln!(self.display, "FOUND AN LDR INSTRUCTION");
                let destination_register = (self.instruction_register as u16 & 0x0E00) >> 9;
                let source_one =
                    self.registers[((self.instruction_register & 0x01C0) >> 6) as usize] as i16;
                let source_two = (((self.instruction_register & 0x3F) << 10) as i16) >> 10;
                let address = (source_one + source_two) as usize & 0xFFFF;
                //write!(self.display, "ADDRESS FOR LDR = {:04X}\r\n", address);

                if address == KBDR {
                    self.memory[KBSR] = 0;
                }

                self.registers[destination_register as usize] = self.memory[address];
                self.update_condition_code(self.registers[destination_register as usize]);
            }
            STR_OPCODE => {
                //writeln!(self.display, "FOUND AN STR INSTRUCTION");
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let source_one =
                    self.registers[((self.instruction_register & 0x01C0) >> 6) as usize] as i16;
                let source_two = (((self.instruction_register & 0x3F) << 10) as i16) >> 10;

                self.memory[(source_one + source_two) as usize] =
                    self.registers[destination_register as usize];
            }
            NOT_OPCODE => {
                //writeln!(self.display, "FOUND A NOT INSTRUCTION");
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let source_one =
                    self.registers[((self.instruction_register & 0x01C0) >> 6) as usize];

                self.registers[destination_register as usize] = !source_one;
                self.update_condition_code(self.registers[destination_register as usize]);
            }
            LDI_OPCODE => {
                //writeln!(self.display, "FOUND AN LDI INSTRUCTION!");
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let offset = (((self.instruction_register & 0x1FF) << 7) as i16) >> 7;
                let address = (self.program_counter as i16 + offset) as usize;
                let indirect = self.memory[address] as usize;

                if indirect == KBDR {
                    self.memory[KBSR] = 0;
                }

                self.registers[destination_register as usize] = self.memory[indirect];
                self.update_condition_code(self.registers[destination_register as usize]);
            }
            STI_OPCODE => {
                //writeln!(self.display, "FOUND AN STI INSTRUCTION");
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let offset = (((self.instruction_register & 0x1FF) << 7) as i16) >> 7;

                self.memory
                    [self.memory[(self.program_counter as i16 + offset) as usize] as usize] =
                    self.registers[destination_register as usize];
            }
            JMP_OPCODE => {
                //writeln!(self.display, "FOUN A JMP INSTRUCTION");
                self.program_counter =
                    self.registers[((self.instruction_register & 0x1C0) >> 6) as usize];
            }
            LEA_OPCODE => {
                //writeln!(self.display, "FOUND AN LEA INSTRUCTION");
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let offset = (((self.instruction_register & 0x1FF) << 7) as i16) >> 7;

                self.registers[destination_register as usize] =
                    (self.program_counter as i16 + offset) as u16;
                self.update_condition_code(self.registers[destination_register as usize]);
            }
            TRAP_OPCODE => {
                //writeln!(self.display, "FOUND A TRAP INSTRUCTION");
                let trap_vector = (self.instruction_register & 0xFF) as usize;
                self.registers[7] = self.program_counter;
                self.program_counter = self.memory[trap_vector];
            }

            RTI_OPCODE | RESERVED_OP => {}
            _ => unreachable!(),
        }
    }
}
