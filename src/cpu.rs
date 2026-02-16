use crate::bus::Bus;
use crate::opcode;
use std::{collections::HashMap};

bitflags! {
    pub struct Status: u8 {
        const CARRY             = 0b00000001;
        const ZERO              = 0b00000010;
        const INTERRUPT_DISABLE = 0b00000100;
        const DECIMAL           = 0b00001000;
        const BREAK             = 0b00010000;
        const UNUSED            = 0b00100000;
        const OVERFLOW          = 0b01000000;
        const NEGATIVE          = 0b10000000;
    }
}

pub struct Cpu {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: Status,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub bus: Bus,
}

#[derive(Debug)]
pub enum AddressingMode {
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    IndirectX,
    IndirectY,
    NoneAdressing,
}

impl Cpu {

    pub fn new(bus: Bus) -> Self {
        Cpu {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: Status::from_bits_truncate(0b100100),
            program_counter: 0,
            stack_pointer: 0xFD,
            bus: bus,
        }
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = Status::from_bits_truncate(0b100100);
        self.program_counter = self.mem_read_le(0xFFFC);
    }

    pub fn stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.mem_read((0x0100 as u16) + self.stack_pointer as u16)
    }

    pub fn stack_pop_le(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;
        hi << 8 | lo
    }

    pub fn stack_push(&mut self, data: u8) {
        self.mem_write(data, (0x0100 as u16) + self.stack_pointer as u16);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1)
    }

    pub fn stack_push_le(&mut self, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xFF) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }

    pub fn branch(&mut self, cond: bool) {
        if cond {
            let jump: i8 = self.mem_read(self.program_counter) as i8;
            let jump_addr = self.program_counter.wrapping_add(1).wrapping_add(jump as u16);

            self.program_counter = jump_addr;
        }
    }

    pub fn mem_read(&mut self, addr: u16) -> u8 {
        self.bus.mem_read(addr)
    }

    pub fn mem_read_le(&mut self, addr: u16) -> u16 {
        let lo = self.mem_read(addr) as u16;
        let hi = self.mem_read(addr + 1) as u16;
        (hi << 8) | (lo as u16) 
    }

    pub fn mem_write(&mut self, value: u8, addr: u16) {
        self.bus.mem_write(value, addr)
    }

    pub fn mem_write_le(&mut self, value: u16, addr: u16) {
        let hi = (value >> 8) as u8;
        let lo = (value & 0xFF) as u8;
        self.mem_write(lo, addr);
        self.mem_write(hi, addr + 1);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        for i in 0..(program.len() as u16) {
            self.mem_write(program[i as usize], 0x0000 + i, );
        }
        self.mem_write_le(0x0000, 0xFFFC);
    }
    
    pub fn load_and_run_pg(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    pub fn get_absolute_address(&mut self, mode: &AddressingMode, addr: u16) -> u16 {
         match mode {
            AddressingMode::ZeroPage => self.mem_read(addr) as u16,
            AddressingMode::Absolute => self.mem_read_le(addr),
            AddressingMode::ZeroPageX => {
                let pos = self.mem_read(addr);
                let addr = pos.wrapping_add(self.register_x) as u16;
                addr
            },
            AddressingMode::ZeroPageY => {
                let pos = self.mem_read(addr);
                let addr = pos.wrapping_add(self.register_y) as u16;
                addr
            },
            AddressingMode::AbsoluteX => {
                let base = self.mem_read_le(addr);
                let addr = base.wrapping_add(self.register_x as u16);
                addr
            },
            AddressingMode::AbsoluteY => {
                let base = self.mem_read_le(addr);
                let addr = base.wrapping_add(self.register_y as u16);
                addr
            },
            AddressingMode::IndirectX => {
                let base = self.mem_read(addr);
                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            },
            AddressingMode::IndirectY => {
                let base = self.mem_read(addr);
                let lo = self.mem_read(base as u16);
                let hi = self.mem_read((base as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let addr = deref_base.wrapping_add(self.register_y as u16);
                addr
            },
            _ => panic!("Mode {:?} is not supported", mode),
        }
    }

    pub fn get_operand_address(&mut self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.program_counter,
            _ => {
                self.get_absolute_address(mode, self.program_counter)
            },
        }
    }

    pub fn compare(&mut self, mode: &AddressingMode, compared: u8) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        
        if compared >= value {
            self.status |= Status::CARRY;
        } else {
            self.status &= !(Status::CARRY)
        }
        if compared == value {
            self.status |= Status::ZERO;
        } else {
            self.status &= !(Status::ZERO);
        }

        if compared.wrapping_sub(value) & Status::NEGATIVE.bits() != 0 {
            self.status |= Status::NEGATIVE;
        } else {
            self.status &= !(Status::NEGATIVE);
        }
    }

    pub fn dec(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        let result = value.wrapping_sub(1);
        self.mem_write(result, addr);
        self.update_zero_negative_flags(result);
    }

    pub fn eor(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_a = self.register_a ^ value;
        self.update_zero_negative_flags(self.register_a);
    }

    pub fn inc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        let result = value.wrapping_add(1);
        self.mem_write(result, addr);
        self.update_zero_negative_flags(result);
    }

    pub fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_negative_flags(self.register_x);
    }

    pub fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_negative_flags(self.register_y);
    }

    pub fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_negative_flags(self.register_x);
    }

    pub fn iny(&mut self) {
        self.register_y = self.register_x.wrapping_add(1);
        self.update_zero_negative_flags(self.register_y);
    }

    pub fn jsr(&mut self) {
        self.stack_push_le(self.program_counter + 2 - 1);
        let target_address = self.mem_read_le(self.program_counter);
        self.program_counter = target_address;
    }

    pub fn pha(&mut self) {
        self.stack_push(self.register_a);
    }

    pub fn php(&mut self) {
        self.stack_push(self.status.bits());
    }

    pub fn pla(&mut self) {
        self.register_a  = self.stack_pop();
        self.update_zero_negative_flags(self.register_a);
    }

    pub fn plp(&mut self) {
        let value = self.stack_pop();

        self.status = Status::from_bits_truncate(value);
        self.status.remove(Status::BREAK);
        self.status.insert(Status::UNUSED);
    }

    pub fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.update_zero_negative_flags(self.register_a);
    }

    pub fn ldx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_x = value;
        self.update_zero_negative_flags(self.register_x);
    }

    pub fn ldy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_y = value;
        self.update_zero_negative_flags(self.register_y);
    }

    pub fn lsr(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        if value & 1 == 1 {
            self.status |= Status::CARRY; 
        } else {
            self.status &= !(Status::CARRY);
        }

        self.mem_write(value >> 1, addr);
        self.update_zero_negative_flags(value >> 1);
    }

    pub fn lsr_accumulator(&mut self) {
        let mut value = self.register_a;
        if value & 1 == 1 {
            self.status |= Status::CARRY;
        } else {
            self.status &= !(Status::CARRY);
        }
        value = value >> 1;
        self.register_a = value;
        self.update_zero_negative_flags(value);
    }

    pub fn ora(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        let result = self.register_a | value;
        self.register_a = result;
        self.update_zero_negative_flags(result);
    }

    pub fn rts(&mut self) {
        self.program_counter = self.stack_pop_le() + 1;
    }

    pub fn sec(&mut self) {
        self.status |= Status::CARRY;
    }

    pub fn clc(&mut self) {
        self.status &= !(Status::CARRY);
    }

    pub fn sed(&mut self) {
        self.status |= Status::DECIMAL;
    }

    pub fn cld(&mut self) {
        self.status &= !(Status::DECIMAL);
    }

    pub fn sei(&mut self) {
        self.status |= Status::INTERRUPT_DISABLE;
    }

    pub fn cli(&mut self) {
        self.status &= !(Status::INTERRUPT_DISABLE);
    }

    pub fn clv(&mut self) {
        self.status &= !(Status::OVERFLOW);
    }

    pub fn rol(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut value = self.mem_read(addr);
        let old_carry = self.status.contains(Status::CARRY);

        if value >> 7 == 1 {
            self.status |= Status::CARRY;
        } else {
            self.status &= !(Status::CARRY);
        }
        value = value << 1;
        if old_carry {
            value |= 1;
        }
        self.mem_write(value, addr);
        self.update_zero_negative_flags(value);
        value
    }

    pub fn rol_accumulator(&mut self) {
        let mut value = self.register_a;
        let old_carry = self.status.contains(Status::CARRY);
        
        if value >> 7 == 1 {
            self.status |= Status::CARRY;
        } else {
            self.status &= !(Status::CARRY);
        }

        value = value << 1;

        if old_carry {
            value = value | 1;
        }
        self.register_a = value;
        self.update_zero_negative_flags(value);
    }

    pub fn ror(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut value = self.mem_read(addr);
        let old_carry = self.status.contains(Status::CARRY);

        if value & 1 == 1 {
            self.status |= Status::CARRY;
        } else {
            self.status &= !(Status::CARRY);
        }
        value = value >> 1;
        if old_carry {
            value = value | Status::NEGATIVE.bits();
        }

        self.mem_write(value, addr);
        self.update_zero_negative_flags(value);
        value
    }

    pub fn ror_accumulator(&mut self) {
        let mut value = self.register_a;
        let old_carry = self.status.bits() & Status::CARRY.bits() != 0;

        if value & 1 == 1 {
            self.status |= Status::CARRY;
        } else {
            self.status &= !(Status::CARRY);
        }
        value = value >> 1;
        if old_carry {
            value = value | Status::NEGATIVE.bits();
        }
        self.register_a = value;
        self.update_zero_negative_flags(value);
    }

    pub fn rti(&mut self) {
        let value = self.stack_pop();

        self.status = Status::from_bits_truncate(value);
        self.status &= !(Status::BREAK);
        self.status |= Status::UNUSED;
        self.program_counter = self.stack_pop_le();
    }

    pub fn sbc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        
        let carry = if self.status.contains(Status::CARRY) { 0 } else { 1 };
        
        let result = (self.register_a as u16)
            .wrapping_sub(value as u16)
            .wrapping_sub(carry as u16);
        
        if result <= 0xFF {
            self.status |= Status::CARRY;
        } else {
            self.status &= !Status::CARRY;
        }
        
        let final_result = result as u8;
        
        if (self.register_a ^ value) & (self.register_a ^ final_result) & 0x80 != 0 {
            self.status |= Status::OVERFLOW;
        } else {
            self.status &= !Status::OVERFLOW;
        }
        
        self.register_a = final_result;
        self.update_zero_negative_flags(self.register_a);
    }

    pub fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(self.register_a, addr);
    }

    pub fn stx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(self.register_x, addr);
    }

    pub fn sty(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(self.register_y, addr);   
    }

    pub fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_negative_flags(self.register_x);
    }

    pub fn tay(&mut self) {
        self.register_y = self.register_a;
        self.update_zero_negative_flags(self.register_y);
    }

    pub fn txa(&mut self) {
        self.register_a = self.register_x;
        self.update_zero_negative_flags(self.register_a);
    }

    pub fn txs(&mut self) {
        self.stack_pointer = self.register_x;
    }

    pub fn tsx(&mut self) {
        self.register_x = self.stack_pointer;
    }

    pub fn tya(&mut self) {
        self.register_a = self.register_y;
        self.update_zero_negative_flags(self.register_a);
    }

    pub fn adc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        
        let carry = if self.status.contains(Status::CARRY) { 1 } else { 0 };
        
        let sum = (self.register_a as u16) + (value as u16) + carry;
        
        if sum > 0xFF {
            self.status |= Status::CARRY;
        } else {
            self.status &= !Status::CARRY;
        }
        
        let result = sum as u8;
        

        if (self.register_a ^ result) & (value ^ result) & 0x80 != 0 {
            self.status |= Status::OVERFLOW;
        } else {
            self.status &= !Status::OVERFLOW;
        }
        
        self.register_a = result;
        self.update_zero_negative_flags(self.register_a);
    }

    pub fn and(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a = self.register_a & value;
        self.update_zero_negative_flags(self.register_a);
    }

    pub fn asl(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let old = self.mem_read(addr);
        self.update_c_flag(old);
        let result = old << 1;
        self.mem_write(result, addr);
        self.update_zero_negative_flags(result);
    }

    pub fn asl_accumulator(&mut self) {
        let mut value = self.register_a;
        if value >> 7 == 1 {
            self.status |= Status::CARRY;
        } else {
            self.status &= !(Status::CARRY);
        }

        value = value << 1;
        self.register_a = value;
        self.update_zero_negative_flags(value);
    }

    pub fn jmp(&mut self) {
        let addr = self.mem_read_le(self.program_counter);
        self.program_counter = addr;
    }

    pub fn jmp_indirect(&mut self) {
        let mem_address = self.mem_read_le(self.program_counter);
        let indirect_ref = if mem_address & 0x00FF == 0x00FF {
            let lo = self.mem_read(mem_address);
            let hi = self.mem_read(mem_address & 0xFFF0);
            (hi as u16) << 8 | (lo as u16)
        } else {
            self.mem_read_le(mem_address)
        };
        self.program_counter = indirect_ref;
    }

    pub fn bit(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        let and = self.register_a & data;

        self.status.set(Status::ZERO, and == 0);
        self.status.set(Status::NEGATIVE, data & 0x80 != 0);
        self.status.set(Status::OVERFLOW, data & 0x40 != 0);
    }

    pub fn update_zero_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.status |= Status::ZERO;
        } else {
            self.status &= !(Status::ZERO);
        }
        if result & Status::NEGATIVE.bits() != 0 {
            self.status |= Status::NEGATIVE;
        } else {
            self.status &= !(Status::NEGATIVE);
        }
    }

    pub fn update_c_flag(&mut self, value: u8) {
        if value & 0x80 != 0 {
            self.status |= Status::CARRY;
        } else {
            self.status &= !(Status::CARRY);
        }
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F) where F: FnMut(&mut Cpu) {
        let ref opcodes: HashMap<u8, &'static opcode::OpCode> = *opcode::OPCODES_MAP;
        loop {
            callback(self);
            let code = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let program_counter_state = self.program_counter;

            let opcode = opcodes.get(&code).expect(&format!("OpCode {:x} is unknown !", code));

            match code {
                /*ADC */
                0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => {
                    self.adc(&opcode.mode);
                },
                /*AND */
                0x29 | 0x25 | 0x35| 0x2D| 0x3D| 0x39| 0x21| 0x31 => {
                    self.and(&opcode.mode);
                },
                /*ASL */
                0x0A => {
                    self.asl_accumulator();
                },
                0x06 | 0x16 | 0x0E | 0x1E => {
                    self.asl(&opcode.mode);
                },
                /*BIT */
                0x24 | 0x2C => {
                    self.bit(&opcode.mode);
                },
                /*CMP */
                0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => {
                    self.compare(&opcode.mode, self.register_a);
                },
                /*CPX */
                0xE0 | 0xE4 | 0xEC => {
                    self.compare(&opcode.mode, self.register_x);
                },
                /*CPY */
                0xC0 | 0xC4 | 0xCC => {
                    self.compare(&opcode.mode, self.register_y);
                },
                /*DEC */
                0xC6 | 0xD6 | 0xCE | 0xDE => {
                    self.dec(&opcode.mode);
                },
                /*EOR */
                0x49 | 0x45 | 0x55 | 0x4D | 0x5D | 0x59 | 0x41 | 0x51 => {
                    self.eor(&opcode.mode);
                },
                /*INC */
                0xE6 | 0xF6 | 0xEE | 0xFE => {
                    self.inc(&opcode.mode);
                },
                /*JMP */
                0x4C => {
                    self.jmp();
                },
                0x6C => {
                    self.jmp_indirect();
                }
                /*LDA */
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
                    self.lda(&opcode.mode);
                },
                /*LDX */
                0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE => {
                    self.ldx(&opcode.mode);
                },
                /*LDY */
                0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC => {
                    self.ldy(&opcode.mode);
                },
                /*LSR */
                0x4A => {
                    self.lsr_accumulator();
                },
                0x46 | 0x56 | 0x4E | 0x5E => {
                    self.lsr(&opcode.mode);
                },
                /*ORA */
                0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => {
                    self.ora(&opcode.mode);
                },
                /*ROL */
                0x2A => {
                    self.rol_accumulator();
                },
                0x26 | 0x36 | 0x2E | 0x3E => {
                    self.rol(&opcode.mode);
                },
                /*ROR */
                0x6A => {
                    self.ror_accumulator();
                },
                0x66 | 0x76 | 0x6E | 0x7E => {
                    self.ror(&opcode.mode);
                },
                /*SBC */
                0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 => {
                    self.sbc(&opcode.mode);
                },
                /*STA */
                0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => {
                    self.sta(&opcode.mode);
                },
                /*STX */
                0x86 | 0x96 | 0x8E => {
                    self.stx(&opcode.mode);
                },
                /*STY */
                0x84 | 0x94 | 0x8C => {
                    self.sty(&opcode.mode);
                },
                0xE8 => self.inx(),
                0xC8 => self.iny(),
                0xAA => self.tax(),
                0xA8 => self.tay(),
                0x8A => self.txa(),
                0x98 => self.tya(),
                0x9A => self.txs(),
                0xBA => self.tsx(),
                0x00 => return,
                0xEA => {},
                0x60 => self.rts(),
                0xF8 => self.sed(),
                0xD8 => self.cld(),
                0x78 => self.sei(),
                0x58 => self.cli(),
                0x38 => self.sec(),
                0x18 => self.clc(),
                0xB8 => self.clv(),
                0x90 => self.branch(!(self.status.contains(Status::CARRY))),
                0xB0 => self.branch(self.status.contains(Status::CARRY)),
                0xF0 => self.branch(self.status.contains(Status::ZERO)),
                0x30 => self.branch(self.status.contains(Status::NEGATIVE)),
                0xD0 => self.branch(!(self.status.contains(Status::ZERO))),
                0x10 => self.branch(!(self.status.contains(Status::NEGATIVE))),
                0x50 => self.branch(!(self.status.contains(Status::OVERFLOW))),
                0x70 => self.branch(self.status.contains(Status::OVERFLOW)),
                0xCA => self.dex(),
                0x88 => self.dey(),
                0x20 => self.jsr(),
                0x48 => self.pha(),
                0x08 => self.php(),
                0x68 => self.pla(),
                0x28 => self.plp(),
                0x40 => self.rti(),
                _ => return,
            }
            if program_counter_state == self.program_counter {
                self.program_counter += (opcode.len - 1) as u16;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::rom::test;

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let bus = Bus::new(test::test_rom(vec![0xa5, 0x10, 0x00]));
        let mut cpu = Cpu::new(bus);
        cpu.load_and_run_pg(vec![0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register_a, 5);
        assert!(!(cpu.status.contains(Status::ZERO)));
        assert!(!(cpu.status.contains(Status::NEGATIVE)));
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let bus = Bus::new(test::test_rom(vec![0xa5, 0x10, 0x00]));
        let mut cpu = Cpu::new(bus);
        cpu.load_and_run_pg(vec![0xa9, 0x00, 0x00]);
        assert!(cpu.status.bits() & Status::ZERO.bits() == 0b10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let bus = Bus::new(test::test_rom(vec![0xa5, 0x10, 0x00]));
        let mut cpu = Cpu::new(bus);
        cpu.load_and_run_pg(vec![0xa9, 0x0A,0xaa, 0x00]);

        assert_eq!(cpu.register_x, 10)
    }

    #[test]
    fn test_5_ops_working_together() {
        let bus = Bus::new(test::test_rom(vec![0xa5, 0x10, 0x00]));
        let mut cpu = Cpu::new(bus);
        cpu.load_and_run_pg(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 0xc1)
    }

    #[test]
    fn test_inx_overflow() {
        let bus = Bus::new(test::test_rom(vec![0xa5, 0x10, 0x00]));
        let mut cpu = Cpu::new(bus);
        cpu.load_and_run_pg(vec![0xa9, 0xff, 0xaa,0xe8, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 1)
    }

    #[test]
    fn test_lda_from_memory() {
        let bus = Bus::new(test::test_rom(vec![0xa5, 0x10, 0x00]));
        let mut cpu = Cpu::new(bus);
        cpu.mem_write( 0x55, 0x10);

        cpu.load_and_run_pg(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_stx_zero_page() {
        let bus = Bus::new(test::test_rom(vec![0xa5, 0x10, 0x00]));
        let mut cpu = Cpu::new(bus);
        cpu.load_and_run_pg(vec![0xA2, 0x0A, 0x86, 0x10, 0x00]);
        assert_eq!(cpu.mem_read(0x10), 0x0A);
    }
    
}
