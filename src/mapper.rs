const PRG_ROM_PAGE_SIZE: usize = 16384;
const CHR_ROM_PAGE_SIZE: usize = 8192;

#[derive(Copy, Clone)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    FourScreen,
    OneScreenUpper,
    OneScreenLower,
}

pub trait Mapper {
    fn get_prg_rom(&self, addr: u16) -> u8;
    fn get_chr_rom(&self, addr: u16) -> u8;
    fn cpu_write(&mut self, addr: u16, value: u8);
    fn ppu_write(&mut self, addr: u16, value: u8);
    fn get_mirroring(&self) -> Mirroring;
    fn get_prg_ram(&self, addr: u16) -> Option<u8>;
    fn prg_ram_write(&mut self, addr: u16, value: u8);
}

pub struct Mapper0 {
    raw: Vec<u8>,
}

impl Mapper0 {
    pub fn new(raw: &Vec<u8>) -> Self {
        Mapper0 { raw: raw.to_vec() }
    }
}

struct Regs1 {
    control: u8,
    chr_bank_0: u8,
    chr_bank_1: u8,
    prg_bank: u8,
}

impl Regs1 {
    fn new() -> Self {
        Regs1 { control: 0xC, chr_bank_0: 0, chr_bank_1: 0, prg_bank: 0 }
    }
}

pub struct Mapper1 {
    registers: Regs1,

    shift_register: u8,
    write_count: u8,
    raw: Vec<u8>,
    chr_ram: Vec<u8>,
    prg_ram: Vec<u8>,
}

impl Mapper1 {
    fn new(raw: &Vec<u8>) -> Self {
        Mapper1 {
            registers: Regs1::new(),
            shift_register: 0,
            write_count: 0,
            raw: raw.to_vec(),
            chr_ram: vec![0; 8192],
            prg_ram: vec![0; 8192 * 32]
        }
    }
}

pub struct Rom {
    pub mapper: Box<dyn Mapper>
}

impl Mapper for Mapper0 {
    fn get_prg_rom(&self, addr: u16) -> u8 {
        let prg_rom_size = self.raw[4] as usize * PRG_ROM_PAGE_SIZE;
        let skip_trainer = self.raw[6] & 0b100 != 0;
        let prg_rom_start = 16 + if skip_trainer { 512 } else { 0 };
        let offset = (addr as usize - 0x8000) % prg_rom_size;
        return self.raw[prg_rom_start + offset];
    }

    fn get_chr_rom(&self, addr: u16) -> u8 {
        let prg_rom_size = self.raw[4] as usize * PRG_ROM_PAGE_SIZE;
        let skip_trainer = self.raw[6] & 0b100 != 0;
        let prg_rom_start = 16 + if skip_trainer { 512 } else { 0 };

        let chr_rom_start = prg_rom_start + prg_rom_size;
        self.raw[chr_rom_start + addr as usize]
    }

    fn get_mirroring(&self) -> Mirroring {
        let four_screen = self.raw[6] & 0b1000 != 0;
        let vertical_mirroring = self.raw[6] & 0b1 != 0;

        match (four_screen, vertical_mirroring) {
            (true,_) => Mirroring::FourScreen,
            (false, true) => Mirroring::Vertical,
            (false, false) => Mirroring::Horizontal,
        }
    }

    fn cpu_write(&mut self, _addr: u16, _value: u8) { }

    fn ppu_write(&mut self, addr: u16, value: u8) {
        let prg_rom_size = self.raw[4] as usize * PRG_ROM_PAGE_SIZE;
        let skip_trainer = self.raw[6] & 0b100 != 0;
        let prg_rom_start = 16 + if skip_trainer { 512 } else { 0 };

        let chr_rom_start = prg_rom_start + prg_rom_size;
        self.raw[chr_rom_start + addr as usize] = value;
    }

    fn get_prg_ram(&self, _addr: u16) -> Option<u8> {
        return None;
    }

    fn prg_ram_write(&mut self, _addr: u16, _value: u8) { }
    
}

impl Mapper for Mapper1 {
    fn get_prg_rom(&self, addr: u16) -> u8 {
        let mode = (self.registers.control >> 2) & 0b11;
        let prg_bank = (self.registers.prg_bank & 0x0F) as usize;
        let prg_rom_banks = self.raw[4] as usize;

        let prg_rom_size = prg_rom_banks * PRG_ROM_PAGE_SIZE;
        let skip_trainer = self.raw[6] & 0b100 != 0;
        let prg_rom_start = 16 + if skip_trainer { 512 } else { 0 };
        let bank_offset = (addr as usize) & 0x3FFF;

        let bank = match mode {
            0 | 1 => (prg_bank & 0x0E) | if addr >= 0xC000 { 1 } else { 0 },
            2 => {
                if addr < 0xC000 { 0 } else { prg_bank }
            }
            3 => {
                if addr < 0xC000 { prg_bank } else { prg_rom_banks - 1 }
            }
            _ => unreachable!(),
        };

        let offset = (bank % prg_rom_banks) * PRG_ROM_PAGE_SIZE + bank_offset;
        self.raw[prg_rom_start + (offset % prg_rom_size)]
    }

    
    fn get_chr_rom(&self, addr: u16) -> u8 {
        let mode = self.registers.control & 0x10 != 0;
        let bank_offset = addr as usize & 0x0FFF;

        let bank = if mode {
            if addr < 0x1000 {
                (self.registers.chr_bank_0 & 0x1F) as usize
            } else {
                (self.registers.chr_bank_1 & 0x1F) as usize
            }
        } else {
            ((self.registers.chr_bank_0 & 0x1E) as usize) * 2
                + ((addr as usize >> 12) & 1)
        };

        let offset = bank * 0x1000 + bank_offset;

        let chr_rom_size = self.raw[5] as usize * CHR_ROM_PAGE_SIZE;

        if chr_rom_size == 0 {
            return self.chr_ram[offset % self.chr_ram.len()];
        }

        let skip_trainer = self.raw[6] & 0x04 != 0;
        let prg_rom_start = 16 + if skip_trainer { 512 } else { 0 };
        let chr_rom_start = prg_rom_start + self.raw[4] as usize * PRG_ROM_PAGE_SIZE;

        self.raw[chr_rom_start + (offset % chr_rom_size)]
    }


    fn get_mirroring(&self) -> Mirroring {
        let nt_arrangement = self.registers.control & 0b11;
        match nt_arrangement {
            0 => Mirroring::OneScreenLower,
            1 => Mirroring::OneScreenUpper,
            2 => Mirroring::Vertical,
            3 => Mirroring::Horizontal,
            _ => unreachable!(),
        }
    }

    fn cpu_write(&mut self, addr: u16, value: u8) {
        if value & 0x80 != 0 {
            self.write_count = 0;
            self.shift_register = 0;
            self.registers.control = self.registers.control | 12;
            return;
        }
        
        self.shift_register |= (value & 1) << self.write_count;
        self.write_count += 1;
        if self.write_count == 5 {
            match addr {
                0x8000..=0x9FFF => {
                    self.registers.control = self.shift_register
                },
                0xA000..=0xBFFF => {
                    self.registers.chr_bank_0 = self.shift_register
                },
                0xC000..=0xDFFF => {
                    self.registers.chr_bank_1 = self.shift_register
                },
                0xE000..=0xFFFF => {
                    self.registers.prg_bank = self.shift_register
                }
                _=> return,
            }
            self.shift_register = 0;
            self.write_count = 0;
        }
    }

    fn ppu_write(&mut self, addr: u16, value: u8) {
        let mode = self.registers.control & 0x10 != 0;
        let bank_offset = addr as usize & 0x0FFF;

        let bank = if mode {
            if addr < 0x1000 {
                (self.registers.chr_bank_0 & 0x1F) as usize
            } else {
                (self.registers.chr_bank_1 & 0x1F) as usize
            }
        } else {
            ((self.registers.chr_bank_0 & 0x1E) as usize) * 2
                + ((addr as usize >> 12) & 1)
        };

        let offset = bank * 0x1000 + bank_offset;

        let chr_rom_size = self.raw[5] as usize * CHR_ROM_PAGE_SIZE;

        if chr_rom_size == 0 {
            let ram_offset = offset % self.chr_ram.len();
            self.chr_ram[ram_offset] = value;
        }
    }

    fn get_prg_ram(&self, addr: u16) -> Option<u8> {
        let prg_ram_enabled: bool = self.registers.prg_bank & 0x10 == 0
            || self.registers.chr_bank_0 & 0x10 == 0
            || self.registers.chr_bank_1 & 0x10 == 0;
        
        if prg_ram_enabled {
            return Some(self.prg_ram[(addr - 0x6000) as usize]);
        } else  {
            return None;
        }
    }

    fn prg_ram_write(&mut self, addr: u16, value: u8) {
        let prg_ram_enabled: bool = self.registers.prg_bank & 0x10 == 0
            || self.registers.chr_bank_0 & 0x10 == 0
            || self.registers.chr_bank_1 & 0x10 == 0;
        
        if prg_ram_enabled {
            self.prg_ram[(addr - 0x6000) as usize] = value;
        }
    }

}

impl Rom {
    pub fn new(raw: &Vec<u8>) -> Result<Self,()> {
        let number = raw[7] & 0b1111_0000 | raw[6] >> 4;
        match number {
            0 => Ok(Rom{ mapper: Box::new(Mapper0::new(raw)) }),
            1 => Ok(Rom{ mapper: Box::new(Mapper1::new(raw))}),
            _ => Err(println!("Number not supported {}", number)),
        }
    }
}
