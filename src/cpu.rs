use crate::opcode;
use std::{collections::HashMap};

pub struct Cpu {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,
    memory: [u8; 0xFFFF],
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
    pub fn new() -> Self {
        Cpu {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: 0,
            program_counter: 0,
            memory: [0; 0xFFFF],
        }
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = 0;
        self.program_counter = self.mem_read_le(0xFFFC);
    }

    pub fn mem_read(&mut self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    pub fn mem_read_le(&mut self, addr: u16) -> u16 {
        let lo = self.mem_read(addr) as u16;
        let hi = self.mem_read(addr + 1) as u16;
        (hi << 8) | (lo as u16) 
    }

    pub fn mem_write(&mut self, value: u8, addr: u16) {
        self.memory[addr as usize] = value;
    }

    pub fn mem_write_le(&mut self, value: u16, addr: u16) {
        let hi = (value >> 8) as u8;
        let lo = (value & 0xFF) as u8;
        self.mem_write(lo, addr);
        self.mem_write(hi, addr + 1);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000 .. (0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_le(0x8000, 0xFFFC);
    }
    
    pub fn load_and_run_pg(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    pub fn get_operand_address(&mut self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Accumulator => {
                panic!("Mode {:?} is not supported", mode)
            },
            AddressingMode::Immediate => self.program_counter,
            AddressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,
            AddressingMode::Absolute => self.mem_read_le(self.program_counter),

            AddressingMode::ZeroPageX => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_x) as u16;
                addr
            },
            AddressingMode::ZeroPageY => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_y) as u16;
                addr
            },
            AddressingMode::AbsoluteX => {
                let base = self.mem_read_le(self.program_counter);
                let addr = base.wrapping_add(self.register_x as u16);
                addr
            },
            AddressingMode::AbsoluteY => {
                let base = self.mem_read_le(self.program_counter);
                let addr = base.wrapping_add(self.register_y as u16);
                addr
            },
            AddressingMode::IndirectX => {
                let base = self.mem_read(self.program_counter);
                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            },
            AddressingMode::IndirectY => {
                let base = self.mem_read(self.program_counter);
                let lo = self.mem_read(base as u16);
                let hi = self.mem_read((base as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let addr = deref_base.wrapping_add(self.register_y as u16);
                addr
            },
            AddressingMode::NoneAdressing => {
                panic!("Mode {:?} is not supported", mode);
            }
        }
    }

    pub fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_negative_flags(self.register_x);
    }

    pub fn iny(&mut self) {
        self.register_y = self.register_x.wrapping_add(1);
        self.update_zero_negative_flags(self.register_y);
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

    pub fn tya(&mut self) {
        self.register_a = self.register_y;
        self.update_zero_negative_flags(self.register_a);
    }

    pub fn and(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_a &= value;
        self.update_zero_negative_flags(self.register_a);
    }

    pub fn asl(&mut self, mode: &AddressingMode) {
        match mode {
            AddressingMode::Accumulator => {
                let old = self.register_a;
                self.update_c_flag(old);
                let result = old << 1;
                self.register_a = result;
                self.update_zero_negative_flags(result);
            },
            _ => {
                let addr = self.get_operand_address(mode);
                let old = self.mem_read(addr);
                self.update_c_flag(old);
                let result = old << 1;
                self.mem_write(result, addr);
                self.update_zero_negative_flags(result);
            }
        }
    }

    pub fn update_zero_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.status |= 0b0000_0010;
        } else {
            self.status &= 0b1111_1101;
        }
        if result & 0b1000_0000 != 0 {
            self.status |= 0b1000_0000;
        } else {
            self.status &= 0b0111_1111;
        }
    }

    pub fn update_c_flag(&mut self, value: u8) {
        if value & 0x80 != 0 {
            self.status |= 0b0000_0001;
        } else {
            self.status &= 0b1111_1110;
        }
    }

    pub fn run(&mut self) {
        let ref opcodes: HashMap<u8, &'static opcode::OpCode> = *opcode::OPCODES_MAP;
        loop {
            let code = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let program_counter_state = self.program_counter;

            let opcode = opcodes.get(&code).expect(&format!("OpCode {:x} is unknown !", code));

            match code {
                /*AND */
                0x29 | 0x25 | 0x35| 0x2D| 0x3D| 0x39| 0x21| 0x31 => {
                    self.and(&opcode.mode);
                },
                /*ASL */
                0x0A | 0x06 | 0x16 | 0x0E | 0x1E => {
                    self.asl(&opcode.mode);
                },
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
                0x00 => return,
                0xEA => {},
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

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = Cpu::new();
        cpu.load_and_run_pg(vec![0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register_a, 5);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = Cpu::new();
        cpu.load_and_run_pg(vec![0xa9, 0x00, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0b10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = Cpu::new();
        cpu.load_and_run_pg(vec![0xa9, 0x0A,0xaa, 0x00]);

        assert_eq!(cpu.register_x, 10)
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = Cpu::new();
        cpu.load_and_run_pg(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 0xc1)
    }

    #[test]
    fn test_inx_overflow() {
        let mut cpu = Cpu::new();
        cpu.load_and_run_pg(vec![0xa9, 0xff, 0xaa,0xe8, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 1)
    }

    #[test]
    fn test_lda_from_memory() {
        let mut cpu = Cpu::new();
        cpu.mem_write( 0x55, 0x10);

        cpu.load_and_run_pg(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_asl_accumulator_basic() {
        let mut cpu = Cpu::new();

        cpu.load_and_run_pg(vec![
            0xA9, 0b0000_0010,
            0x0A,
            0x00,
        ]);

        assert_eq!(cpu.register_a, 0b0000_0100);
        assert_eq!(cpu.status & 0b0000_0001, 0);
        assert_eq!(cpu.status & 0b0000_0010, 0);
        assert_eq!(cpu.status & 0b1000_0000, 0);
    }
    #[test]
    fn test_stx_zero_page() {
        let mut cpu = Cpu::new();
        cpu.load_and_run_pg(vec![0xA2, 0x0A, 0x86, 0x10, 0x00]);
        assert_eq!(cpu.mem_read(0x10), 0x0A);
    }
}
