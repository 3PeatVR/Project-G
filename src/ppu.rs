use core::panic;

use crate::rom::Mirroring;
use bitflags::bitflags;

pub struct Ppu {
    pub chr_rom: Vec<u8>,
    pub vram: [u8; 2048],
    pub internal_data_buff: u8,

    pub palettes: [u8; 32],
    pub oam: [u8; 256],
    pub oam_addr: u8,
    pub addr: AddrRegisters,

    pub mirroring: Mirroring,
    pub ctrl: PpuController,
    pub mask: PpuMask,
    pub scroll: PpuScroll,
    pub status: PpuStatus,
    pub scanline: u16,
    pub cycles: usize,
    pub nmi_interrupt: Option<()>,
}

pub struct AddrRegisters {
    addresses: (u8, u8),
    hi_ptr: bool,
}

pub struct PpuScroll {
    pub scroll_x: u8,
    pub scroll_y: u8,

    pub has_scroll: bool,
}

bitflags! {
    pub struct PpuController: u8 {
        const NAMETABLE1 = 0b0000_0001;
        const NAMETABLE2 = 0b0000_0010;
        const VRAM_INCREMENT = 0b0000_0100;
        const SPRITE_PATTERN = 0b0000_1000;
        const BG_PATTERN = 0b0001_0000;
        const SPRITE_SIZE = 0b0010_0000;
        const MASTER_SLAVE_SELECT = 0b0100_0000;
        const NMI = 0b1000_0000;
    }
    pub struct PpuMask: u8 {
        const GRAYSCALE = 0b0000_0001;
        const SHOW_BG_LEFT = 0b0000_0010;
        const SHOW_SPRITES_LEFT = 0b0000_0100;
        const ENABLE_BACKGROUND = 0b0000_1000;
        const ENABLE_SPRITES = 0b0001_0000;
        const EMPHASIZE_RED = 0b0010_0000;
        const EMPHASIZE_GREEN = 0b0100_0000;
        const EMPHASIZE_BLUE = 0b1000_0000;
    }

    pub struct PpuStatus: u8 {
        const SPRITE_OVERFLOW_FLAG = 0b0010_0000;
        const SRITE_0_FLAG = 0b0100_0000;
        const VBLANK_FLAG = 0b1000_0000;
    }

}

impl PpuController {
    pub fn new() -> Self {
        PpuController::from_bits_truncate(0b0000_0000)
    }

    pub fn write_ctrl(&mut self, data: u8) {
        *self = PpuController::from_bits_truncate(data);  
    }
    
    pub fn increment_vram(&mut self) -> u8 {
        if !self.contains(PpuController::VRAM_INCREMENT) {
            1
        } else {
            32
        }
    }

    pub fn generate_vblank_nmi(&self) -> bool {
        self.contains(PpuController::NMI)
    }

    pub fn get_bg_bank(&self) -> u16 {
        let result = self.bits() & 0b11;
        match result {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => panic!("cannot be")
        }
    }

    pub fn sprite_pattern_addr(&self) -> u16 {
        if !self.contains(PpuController::SPRITE_PATTERN) {
            0
        } else {
            0x1000
        }
    }
}

impl PpuMask {
    pub fn new() -> Self {
        PpuMask::from_bits_truncate(0b0000_0000)
    }

    pub fn write_mask(&mut self, data: u8) {
        *self = PpuMask::from_bits_truncate(data);  
    }
}

impl PpuScroll {
    pub fn new() -> Self {
        PpuScroll { scroll_x: 0, scroll_y: 0, has_scroll: false }
    }

    pub fn write_scroll(&mut self, data: u8) {
        if !self.has_scroll {
            self.scroll_x = data;
        } else {
            self.scroll_y = data;
        }
        self.has_scroll = !self.has_scroll;
    }
}

impl PpuStatus {
    pub fn reset_vblank_status(&mut self) {
        self.remove(PpuStatus::VBLANK_FLAG);
    }

    pub fn set_vblank_status(&mut self, should_set: bool) {
        if should_set {
            self.insert(PpuStatus::VBLANK_FLAG);
        } else {
            self.reset_vblank_status();
        }
    }

    pub fn is_in_vblank(&self) -> bool {
        self.contains(PpuStatus::VBLANK_FLAG)
    }
}

impl AddrRegisters {
    pub fn new() -> Self {
        AddrRegisters { addresses: (0, 0), hi_ptr: true }
    }

    pub fn set(&mut self, data: u16) {
        self.addresses.0 = (data >> 8) as u8;
        self.addresses.1= (data & 0xFF) as u8;

        if self.get() > 0x3FFF {
            self.set(self.get() & 0xFFFF);
        }
        self.hi_ptr = ! self.hi_ptr;
    }

    pub fn update(&mut self, data: u8) {
        if self.hi_ptr {
            self.addresses.0 = data;
        } else {
            self.addresses.1 = data;
        }

        self.hi_ptr = !self.hi_ptr;
    }

    pub fn increment(&mut self, inc: u8) {
        let lo = self.addresses.1;
        self.addresses.1 = self.addresses.1.wrapping_add(inc);
        if lo > self.addresses.1 {
            self.addresses.0 = self.addresses.0.wrapping_add(1);
        }
        if self.get() > 0x3FFF {
            self.set(self.get() & 0xFFFF);
        }
    }

    pub fn reset_ptr(&mut self) {
        self.hi_ptr = true;
    }

    pub fn get(&self) -> u16 {
        ((self.addresses.0 as u16) << 8) | (self.addresses.1 as u16)
    }
}

impl Ppu {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        Ppu {
            chr_rom: chr_rom,
            vram: [0; 2048],
            palettes: [0; 32],
            oam: [0; 256],
            oam_addr: 0,
            mirroring: mirroring,
            addr: AddrRegisters::new(),
            ctrl: PpuController::new(),
            mask: PpuMask::new(),
            scroll: PpuScroll::new(),
            status: PpuStatus::from_bits_truncate(0),
            scanline: 0,
            cycles: 0,
            nmi_interrupt: None,
            internal_data_buff: 0,
        }
    }

    pub fn write_oam_addr(&mut self, data: u8) {
        self.oam_addr = data;
    }

    pub fn write_oam_data(&mut self, data: u8) {
        self.oam[self.oam_addr as usize] = data;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    pub fn read_oam_data(&self) -> u8 {
        self.oam[self.oam_addr as usize]
    }

    pub fn write_addr_registers(&mut self, data: u8) {
        self.addr.update(data);
    }

    pub fn write_to_ctrl(&mut self, data: u8) {
        let before_nmi_status = self.ctrl.generate_vblank_nmi();
        self.ctrl.write_ctrl(data);
        if !before_nmi_status && self.ctrl.generate_vblank_nmi() && self.status.is_in_vblank() {
            self.nmi_interrupt = Some(());
        }
    }

    pub fn read_status(&mut self) -> u8 {
        let data = self.status.bits();
        self.status.remove(PpuStatus::VBLANK_FLAG);
        self.scroll.has_scroll = false;
        self.addr.hi_ptr = true;
        return data;
    }

    pub fn write_to_mask(&mut self, data: u8) {
        self.mask.write_mask(data);
    }

    pub fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.increment_vram());
    }

    pub fn read_data(&mut self) -> u8 {
        let addr = self.addr.get();
        self.increment_vram_addr();

        match addr {
            0..=0x1FFF => {
                let result = self.internal_data_buff;
                self.internal_data_buff = self.chr_rom[addr as usize];
                return result;
            },
            0x2000..=0x2FFF => {
                let result = self.internal_data_buff;
                self.internal_data_buff = self.vram[self.mirror_vram_addr(addr) as usize];
                return result;
            },
            0x3000..=0x3EFF => panic!("Addresses in 0x3000..0x3EFF are not expected"),
            0x3F00..=0x3FFF => {
                self.palettes[(addr - 0x3F00) as usize]
            },
            _ => panic!("Unexpected address {}", addr),
        }
    }

    pub fn write_data(&mut self, value: u8) {
        let addr = self.addr.get();
        match addr  {
            0x2000..=0x2FFF => {
                let mirrored = self.mirror_vram_addr(addr);
                self.vram[mirrored as usize] = value;
            },
            0x3000..=0x3EFF => {
                let mirrored = self.mirror_vram_addr(addr - 0x1000);
                self.vram[mirrored as usize] = value;
            },
            0x3F00..=0x3FFF => {
                let palette_addr = (addr - 0x3F00) % 0x20;
                self.palettes[palette_addr as usize] = value;
            },
            _ => {}
        }
        self.increment_vram_addr();
    }

    pub fn mirror_vram_addr(&mut self, addr: u16) -> u16 {
        let mirrored_vram = addr & 0x2FFF;
        let vram_index = mirrored_vram - 0x2000;
        let name_table = vram_index / 0x400;

        match(&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 1) | (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }

    pub fn tick(&mut self) {
        self.cycles += 1;
        if self.scanline == 241 && self.cycles == 1 {
            self.status.set_vblank_status(true);

            if self.ctrl.generate_vblank_nmi() {
                self.nmi_interrupt = Some(());
            }
        }

        if self.scanline == 261 && self.cycles == 1 {
            self.status.reset_vblank_status();
        }

        if self.cycles >= 341 {
            if self.is_sprite_0_hit(self.cycles) {
                self.status.set(PpuStatus::SRITE_0_FLAG, true);
            }
            self.cycles = 0;
            self.scanline += 1;

            if self.scanline >= 262 {
                self.scanline = 0;
                self.nmi_interrupt = None;
                self.status.set(PpuStatus::SRITE_0_FLAG, false);
                self.status.reset_vblank_status();
            }
        }
    }

    fn is_sprite_0_hit(&self, cycle: usize) -> bool {
        let x = self.oam[0] as usize;
        let y = self.oam[3] as usize;
        (y == self.scanline as usize) && x <= cycle && self.mask.contains(PpuMask::SHOW_SPRITES_LEFT)
    }
}