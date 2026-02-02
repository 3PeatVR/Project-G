const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1FFF;
const PPU_REGISTERS: u16 = 0x2000;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;

pub struct Bus {
    cpu_vram: [u8; 2048]
}

impl Bus {
    pub fn new() -> Self {
        Bus {
            cpu_vram: [0; 2048]
        }
    }
}

impl Mem for Bus {
    fn mem_read(&self, addr: u16) -> u8 {
        match addr {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00000111_11111111;
                self.cpu_vram[mirro_down_addr as usize]
            },
            PPU_REGISTERS ..= PPU_REGISTERS_MIRRORS_END => {
                let _mirror_down_addr = addr & 0b00100000_00000111;
                // TODO implement PPU
            },
            _ => {
                println!("Ignoring mem access at {}", addr);
                0
            }

        }
    }

    fn mem_write(&mut self, value: u8, addr: u16) {
        match addr {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b11111111111;
                self.cpu_vram[mirro_down_addr as usize] = value;
            },
            PPU_REGISTERS ..= PPU_REGISTERS_MIRRORS_END => {
                let _mirror_down_addr = addr & 0b00100000_00000111;
                // TODO implement PPU
            },
            _ => {
                println!("Ignoring memory writing for {}", addr);
            }
        }
    }
}