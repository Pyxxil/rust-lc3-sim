use super::prediction::Branch;
use super::Simulator;

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

#[derive(Debug, Copy, Clone)]
pub enum Instruction {
    Branch(usize, i16),
    Add(usize, usize, bool, i16),
    Load(usize, i16),
    Store(usize, i16),
    JumpSubroutine(bool, i16),
    And(usize, usize, bool, i16),
    LoadRelative(usize, usize, i16),
    StoreRelative(usize, usize, i16),
    ReturnFromInterrupt(u16),
    Not(usize, usize, u16),
    LoadIndirect(usize, i16),
    StoreIndirect(usize, i16),
    Jump(u16, usize, u16),
    Reserved(u16),
    LoadEffectiveAddress(usize, i16),
    Trap(u16, u16),
}

const fn sign_extend(val: u16, length: u16) -> i16 {
    (val << (16 - length)) as i16 >> (16 - length)
}

impl Instruction {
    pub fn branches(&self) -> bool {
        match *self {
            Self::Branch(_, _)
            | Self::JumpSubroutine(_, _)
            | Self::Jump(_, _, _)
            | Self::Trap(_, _) => true,
            _ => false,
        }
    }

    pub fn execute(self, simulator: &mut Simulator) -> (Branch, Self) {
        match self {
            Self::Branch(nzp, offset) => {
                if nzp & simulator.cc != 0 {
                    simulator.pc = (simulator.pc as i16 + offset) as u16;
                    (Branch::Taken, self)
                } else {
                    (Branch::NotTaken, self)
                }
            }
            Self::Add(destination, source_one, from_register, source_two) => {
                simulator.write_register(
                    destination,
                    (simulator.read_register(source_one) as i16).wrapping_add(if from_register {
                        simulator.read_register((source_two & 0x7) as usize) as i16
                    } else {
                        source_two
                    }) as u16,
                );
                (Branch::None, self)
            }
            Self::Load(destination, offset) => {
                let value = simulator.read_memory((simulator.pc as i16 + offset) as u16);
                simulator.write_register(destination, value);
                (Branch::None, self)
            }
            Self::Store(source, offset) => {
                simulator.write_memory(
                    (simulator.pc as i16 + offset) as u16,
                    simulator.read_register(source),
                );
                (Branch::None, self)
            }
            Self::JumpSubroutine(from_register, offset) => {
                simulator.write_register_no_update(7, simulator.pc);
                simulator.pc = if from_register {
                    simulator.read_register(((offset & 0x01C0) >> 6) as usize)
                } else {
                    (simulator.pc as i16 + offset) as u16
                };
                (Branch::Jump, self)
            }
            Self::And(destination, source_one, from_register, source_two) => {
                simulator.write_register(
                    destination,
                    ((simulator.read_register(source_one) as i16)
                        & (if from_register {
                            simulator.read_register((source_two & 0x7) as usize) as i16
                        } else {
                            source_two
                        })) as u16,
                );
                (Branch::None, self)
            }
            Self::LoadRelative(destination, source, offset) => {
                let value =
                    simulator.read_memory((simulator.read_register(source) as i16 + offset) as u16);
                simulator.write_register(destination, value);
                (Branch::None, self)
            }
            Self::StoreRelative(source_one, source_two, offset) => {
                simulator.write_memory(
                    (simulator.read_register(source_two) as i16 + offset) as u16,
                    simulator.read_register(source_one),
                );
                (Branch::None, self)
            }
            Self::Not(destination, source, _) => {
                simulator.write_register(destination, !simulator.read_register(source));
                (Branch::None, self)
            }
            Self::LoadIndirect(destination, offset) => {
                let indirect = simulator.read_memory((simulator.pc as i16 + offset) as u16);
                let value = simulator.read_memory(indirect);
                simulator.write_register(destination, value);
                (Branch::None, self)
            }
            Self::StoreIndirect(source, offset) => {
                let indirect = simulator.read_memory((simulator.pc as i16 + offset) as u16);
                simulator.write_memory(indirect, simulator.read_register(source));
                (Branch::None, self)
            }
            Self::Jump(_, register, _) => {
                simulator.pc = simulator.read_register(register);
                (Branch::Jump, self)
            }
            Self::LoadEffectiveAddress(destination, offset) => {
                simulator.write_register(destination, (simulator.pc as i16 + offset) as u16);
                (Branch::None, self)
            }
            Self::Trap(_, vector) => {
                simulator.write_register_no_update(7, simulator.pc);
                simulator.pc = simulator.read_memory(vector);
                (Branch::Jump, self)
            }
            Self::ReturnFromInterrupt(_) | Self::Reserved(_) => (Branch::None, self),
        }
    }
}

impl From<u16> for Instruction {
    fn from(instruction: u16) -> Self {
        let opcode = instruction & 0xF000;

        let destination_register = usize::from(instruction >> 9 & 0b111);
        let source_register_one = usize::from(instruction >> 6 & 0b111);

        let pc_offset_9 = sign_extend(instruction, 9);
        let offset_6 = sign_extend(instruction, 6);
        let imm5 = sign_extend(instruction, 5);

        match opcode {
            OPCODE_BR => Self::Branch(destination_register, pc_offset_9),
            OPCODE_ADD => Self::Add(
                destination_register,
                source_register_one,
                (instruction & 0x20) == 0,
                imm5,
            ),
            OPCODE_LD => Self::Load(destination_register, pc_offset_9),
            OPCODE_ST => Self::Store(destination_register, pc_offset_9),
            OPCODE_JSR => {
                Self::JumpSubroutine((instruction & 0x0800) == 0, sign_extend(instruction, 11))
            }
            OPCODE_AND => Self::And(
                destination_register,
                source_register_one,
                (instruction & 0x20) == 0,
                imm5,
            ),
            OPCODE_LDR => Self::LoadRelative(destination_register, source_register_one, offset_6),
            OPCODE_STR => Self::StoreRelative(destination_register, source_register_one, offset_6),
            OPCODE_RTI => Self::ReturnFromInterrupt(instruction & 0x0FFF),
            OPCODE_NOT => Self::Not(
                destination_register,
                source_register_one,
                (offset_6 & 0x3F) as u16,
            ),
            OPCODE_LDI => Self::LoadIndirect(destination_register, pc_offset_9),
            OPCODE_STI => Self::StoreIndirect(destination_register, pc_offset_9),
            OPCODE_JMP => Self::Jump(
                instruction & 0x0E00,
                source_register_one,
                (offset_6 & 0x3F) as u16,
            ),
            RESERVED => Self::Reserved(instruction & 0x0FFF),
            OPCODE_LEA => Self::LoadEffectiveAddress(destination_register, pc_offset_9),
            OPCODE_TRAP => Self::Trap(instruction & 0x0F00, instruction & 0x00FF),
            _ => unreachable!(),
        }
    }
}

impl From<Instruction> for u16 {
    fn from(instruction: Instruction) -> u16 {
        match instruction {
            Instruction::Branch(nzp, offset) => (nzp << 9) as u16 | (offset & 0x1FF) as u16,
            Instruction::Add(destination, source_one, from_register, source_two) => {
                0x1000
                    | (destination << 9) as u16
                    | (source_one << 6) as u16
                    | (if from_register { 0x0 } else { 0x20 })
                    | (source_two & 0x1F) as u16
            }
            Instruction::Load(destination, offset) => {
                0x2000 | ((destination << 9) as u16) | (offset & 0x1FF) as u16
            }
            Instruction::Store(source, offset) => {
                0x3000 | (source << 9) as u16 | (offset & 0x1FF) as u16
            }
            Instruction::JumpSubroutine(from_register, offset) => {
                0x4000
                    | if from_register {
                        (offset & 0x01C0) >> 6
                    } else {
                        0x0800 | (offset & 0x7FF)
                    } as u16
            }
            Instruction::And(destination, source_one, from_register, source_two) => {
                0x5000
                    | (destination << 9) as u16
                    | (source_one << 6) as u16
                    | (if from_register { 0x0 } else { 0x20 })
                    | (source_two & 0x1F) as u16
            }
            Instruction::LoadRelative(destination, source, offset) => {
                0x6000 | (destination << 9) as u16 | (source << 6) as u16 | (offset & 0x3F) as u16
            }
            Instruction::StoreRelative(source_one, source_two, offset) => {
                0x7000
                    | (source_one << 9) as u16
                    | (source_two << 6) as u16
                    | (offset & 0x3F) as u16
            }
            Instruction::ReturnFromInterrupt(extra) => 0x8000 | extra,
            Instruction::Not(destination, source, bit6one) => {
                0x9000 | (destination << 9) as u16 | (source << 6) as u16 | bit6one
            }
            Instruction::LoadIndirect(destination, offset) => {
                0xA000 | (destination << 9) as u16 | (offset & 0x1FF) as u16
            }
            Instruction::StoreIndirect(source, offset) => {
                0xB000 | (source << 9) as u16 | (offset & 0x1FF) as u16
            }
            Instruction::Jump(bit3zero, register, bit6zero) => {
                0xC000 | bit3zero | (register << 6) as u16 | bit6zero
            }
            Instruction::Reserved(extra) => 0xD000 | extra,
            Instruction::LoadEffectiveAddress(destination, offset) => {
                0xE000 | (destination << 9) as u16 | offset as u16
            }
            Instruction::Trap(extra, vector) => 0xF000 | extra | (vector & 0x00FF),
        }
    }
}
