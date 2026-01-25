pub struct Cpu {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,
    memory: [u8; 0xFFFF],
}

#[derive(Debug)]
pub enum AdressingMode {
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

    pub fn get_operand_address(&mut self, mode: &AdressingMode) -> u16 {
        match mode {
            AdressingMode::Immediate => self.program_counter,
            AdressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,
            AdressingMode::Absolute => self.mem_read_le(self.program_counter),

            AdressingMode::ZeroPageX => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_x) as u16;
                addr
            },
            AdressingMode::ZeroPageY => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_y) as u16;
                addr
            },
            AdressingMode::AbsoluteX => {
                let base = self.mem_read_le(self.program_counter);
                let addr = base.wrapping_add(self.register_x as u16);
                addr
            },
            AdressingMode::AbsoluteY => {
                let base = self.mem_read_le(self.program_counter);
                let addr = base.wrapping_add(self.register_y as u16);
                addr
            },
            AdressingMode::IndirectX => {
                let base = self.mem_read(self.program_counter);
                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            },
            AdressingMode::IndirectY => {
                let base = self.mem_read(self.program_counter);
                let lo = self.mem_read(base as u16);
                let hi = self.mem_read((base as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let addr = deref_base.wrapping_add(self.register_y as u16);
                addr
            },
            AdressingMode::NoneAdressing => {
                panic!("Mode {:?} is not supported", mode);
            }
        }
    }

    pub fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_negative_flags(self.register_x);
    }

    pub fn lda(&mut self, mode: &AdressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.update_zero_negative_flags(self.register_a);
    }

    pub fn sta(&mut self, mode: &AdressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(self.register_a, addr);
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

    pub fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_negative_flags(self.register_x);
    }

    pub fn run(&mut self) {
        loop {
            let op_code = self.mem_read(self.program_counter);
            self.program_counter += 1;

            match op_code {
                0xA9 => {
                    self.lda(&AdressingMode::Immediate);
                    self.program_counter += 1;
                },
                0xA5 => {
                    self.lda(&AdressingMode::ZeroPage);
                    self.program_counter += 1;
                },
                0xAD => {
                    self.lda(&AdressingMode::Absolute);
                    self.program_counter += 2;
                },
                0x00 => {
                    return;
                }
                0xAA => {
                    self.tax();
                    self.program_counter += 1;
                },
                0xE8 => {
                    self.inx();
                    self.program_counter += 1;
                },
                0x85 => {
                    self.sta(&AdressingMode::ZeroPage);
                    self.program_counter += 1;
                },
                0x95 => {
                    self.sta(&AdressingMode::ZeroPageX);
                    self.program_counter += 1;
                }
                _ => {
                    return;
                }
            }
        }
    }
}
