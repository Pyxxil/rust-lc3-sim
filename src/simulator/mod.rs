use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter, Read, Write};
use std::str;

use crossterm::{AsyncReader, InputEvent, KeyEvent};

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

pub enum Reader {
    Keyboard(AsyncReader),
    InFile(BufReader<File>),
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        match self {
            Reader::Keyboard(ref mut reader) => {
                if let Some(key) = reader.next() {
                    if let InputEvent::Keyboard(k) = key {
                        if let KeyEvent::Char(c) = k {
                            buf[0] = c as u8;
                            return Ok(1);
                        }
                    }
                }
            }
            Reader::InFile(ref mut file) => {
                return match file.read_exact(buf) {
                    Ok(_) => Ok(1),
                    _ => Ok(0),
                };
            }
        }

        Ok(0)
    }
}

pub enum Writer {
    Terminal(crossterm::Terminal),
    OutFile(BufWriter<File>),
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = str::from_utf8(&buf).unwrap();
        match self {
            Writer::Terminal(ref mut terminal) => match terminal.write(s) {
                _ => {}
            },
            Writer::OutFile(ref mut file) => match write!(file, "{}", s) {
                _ => {}
            },
        }

        Ok(s.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub enum Tracer {
    NoTrace,
    TraceFile(BufWriter<File>, u16, bool),
}

trait Trace {
    fn wants(&self, instruction: u16, pc: u16) -> bool;
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

pub struct Simulator {
    memory: [u16; 0xFFFF],
    registers: [u16; 8],
    program_counter: u16,
    instruction_register: u16,
    condition_code: char,
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
            program_counter: 0,
            instruction_register: 0,
            condition_code: 'Z',
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

        let mut addr = u16::from(buffer[0]) << 8 | u16::from(buffer[1]);

        self.program_counter = addr;

        (2..buffer.len()).step_by(2).for_each(|i| {
            self.memory[addr as usize] = u16::from(buffer[i]) << 8 | u16::from(buffer[i + 1]);
            addr += 1;
        });

        Ok(self)
    }

    fn update_condition_code(&mut self, value: u16) {
        self.condition_code = if value == 0 {
            'Z'
        } else if value & 0x8000 == 0 {
            'P'
        } else {
            'N'
        };
    }

    fn fetch(&mut self) {
        self.instruction_register = self.memory[self.program_counter as usize];
        self.program_counter = self.program_counter.wrapping_add(1);
    }

    fn trace(&mut self) {
        if self.tracer.wants(
            (self.instruction_register & 0xF000) >> 12,
            self.program_counter,
        ) {
            self.tracer.trace(
                format!(
                    "After executing instruction: 0x{:04X}\n{}Program Counter: 0x{:04X}\nCondition Code: {}\n===================================\n",
                    self.instruction_register,
                    (0..8)
                        .map(|i| format!("Register {}: 0x{:04X}\n", i, self.registers[i]))
                        .collect::<String>(),
                    self.program_counter,
                    self.condition_code
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
                let _ = self
                    .display
                    .write(
                        format!(
                            "{}{}",
                            if value & 0xFF == 0xA { "\r" } else { "" },
                            (value & 0xFF) as u8 as char
                        )
                        .as_ref(),
                    )
                    .unwrap();
                self.memory[DDR] = 0;
                self.memory[DSR] = 0x8000;
            }
            addr => {
                self.memory[addr] = value;
            }
        }
    }

    fn step(&mut self) {
        let opcode = self.instruction_register & 0xF000;

        match opcode {
            BR_OPCODE => {
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
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
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
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let offset = (((self.instruction_register & 0x1FF) << 7) as i16) >> 7;
                let address = (self.program_counter as i16 + offset) as u16;

                self.registers[destination_register as usize] = self.read_memory(address);
                self.update_condition_code(self.registers[destination_register as usize]);
            }
            ST_OPCODE => {
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let offset = (((self.instruction_register & 0x1FF) << 7) as i16) >> 7;
                let address = (self.program_counter as i16 + offset) as u16;

                self.write_memory(address, self.registers[destination_register as usize]);
            }
            JSR_OPCODE => {
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
                let destination_register = (self.instruction_register as u16 & 0x0E00) >> 9;
                let source_one =
                    self.registers[((self.instruction_register & 0x01C0) >> 6) as usize] as i16;
                let source_two = (((self.instruction_register & 0x3F) << 10) as i16) >> 10;
                let address = (source_one + source_two) as u16;

                self.registers[destination_register as usize] = self.read_memory(address);
                self.update_condition_code(self.registers[destination_register as usize]);
            }
            STR_OPCODE => {
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let source_one =
                    self.registers[((self.instruction_register & 0x01C0) >> 6) as usize] as i16;
                let source_two = (((self.instruction_register & 0x3F) << 10) as i16) >> 10;
                let address = (source_one + source_two) as u16;

                self.write_memory(address, self.registers[destination_register as usize]);
            }
            NOT_OPCODE => {
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let source_one =
                    self.registers[((self.instruction_register & 0x01C0) >> 6) as usize];

                self.registers[destination_register as usize] = !source_one;
                self.update_condition_code(self.registers[destination_register as usize]);
            }
            LDI_OPCODE => {
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let offset = (((self.instruction_register & 0x1FF) << 7) as i16) >> 7;
                let address = (self.program_counter as i16 + offset) as u16;
                let indirect = self.read_memory(address);

                self.registers[destination_register as usize] = self.read_memory(indirect);
                self.update_condition_code(self.registers[destination_register as usize]);
            }
            STI_OPCODE => {
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let offset = (((self.instruction_register & 0x1FF) << 7) as i16) >> 7;
                let address = (self.program_counter as i16 + offset) as u16;
                let indirect = self.read_memory(address);

                self.write_memory(indirect, self.registers[destination_register as usize]);
            }
            JMP_OPCODE => {
                self.program_counter =
                    self.registers[((self.instruction_register & 0x1C0) >> 6) as usize];
            }
            LEA_OPCODE => {
                let destination_register = (self.instruction_register & 0x0E00) >> 9;
                let offset = (((self.instruction_register & 0x1FF) << 7) as i16) >> 7;

                self.registers[destination_register as usize] =
                    (self.program_counter as i16 + offset) as u16;
                self.update_condition_code(self.registers[destination_register as usize]);
            }
            TRAP_OPCODE => {
                let trap_vector = (self.instruction_register & 0xFF) as usize;
                self.registers[7] = self.program_counter;
                self.program_counter = self.memory[trap_vector];
            }

            RTI_OPCODE | RESERVED_OP => {}
            _ => unreachable!(),
        }
    }
}
