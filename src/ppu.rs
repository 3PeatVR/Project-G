use core::panic;

use crate::rom::Mirroring;
use bitflags::bitflags;

pub struct Ppu {
    pub chr_rom: Vec<u8>,
    pub vram: [u8; 4096],
    pub internal_data_buff: u8,

    pub palettes: [u8; 32],
    pub oam: [u8; 256],
    pub oam_addr: u8,

    pub mirroring: Mirroring,
    pub ctrl: PpuController,
    pub mask: PpuMask,
    pub status: PpuStatus,
    pub scanline: u16,
    pub cycles: usize,
    pub nmi_interrupt: Option<()>,

    pub w_latch: bool,
    pub v: u16,
    pub t: u16,
    pub fine_x: u8,
    line_v: u16,
    pub frame_pixels: Vec<u8>,
    pub frame_ready: bool,
    sprite_zero_dot: Option<usize>,
    chr_is_ram: bool,
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

impl Ppu {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        let chr_is_ram = chr_rom.is_empty();
        let chr_rom = if chr_is_ram {
            vec![0; 8192]
        } else  {
            chr_rom
        };
        Ppu {
            chr_rom,
            chr_is_ram,
            vram: [0; 4096],
            palettes: [0; 32],
            oam: [0; 256],
            oam_addr: 0,
            mirroring: mirroring,
            ctrl: PpuController::new(),
            mask: PpuMask::new(),
            status: PpuStatus::from_bits_truncate(0),
            scanline: 261,
            cycles: 0,
            nmi_interrupt: None,
            internal_data_buff: 0,
            w_latch: false,
            v: 0,
            t: 0,
            fine_x: 0,
            line_v: 0,
            frame_pixels: vec![0; 256 * 240],
            frame_ready: false,
            sprite_zero_dot: None,
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
        if !self.w_latch {
            self.t = (self.t & 0x00FF) | (((data & 0x3F) as u16) << 8);
        } else {
            self.t = (self.t & 0x7F00) | data as u16;
            self.v = self.t;
        }
        self.w_latch = !self.w_latch;
    }

    pub fn write_to_ctrl(&mut self, data: u8) {
        let before_nmi_status = self.ctrl.generate_vblank_nmi();

        self.ctrl.write_ctrl(data);
        self.t = (self.t & !0x0C00) | (((data & 0x03) as u16) << 10);

        if !before_nmi_status
            && self.ctrl.generate_vblank_nmi()
            && self.status.is_in_vblank()
        {
            self.nmi_interrupt = Some(());
        }
    }

    pub fn read_status(&mut self) -> u8 {
        let data = self.status.bits();
        self.status.remove(PpuStatus::VBLANK_FLAG);
        self.w_latch = false;
        return data;
    }

    pub fn write_to_mask(&mut self, data: u8) {
        self.mask.write_mask(data);
    }

    pub fn increment_vram_addr(&mut self) {
        if self.rendering_enabled() && self.rendering_scanline() {
            self.increment_x();
            self.increment_y();
        } else {
            let increment = self.ctrl.increment_vram() as u16;
            self.v = self.v.wrapping_add(increment) & 0x7FFF;
        }
    }
    pub fn write_scroll(&mut self, data: u8) {
        if !self.w_latch {
            self.t = (self.t & !0x001F) | ((data as u16) >> 3);
            self.fine_x = data & 7;
        } else {
            self.t = (self.t & !0x73E0)
                | (((data as u16) & 0xF8) << 2)
                | (((data as u16) & 0x07) << 12);
        }
        self.w_latch = !self.w_latch;
    }

    pub fn read_data(&mut self) -> u8 {
        let addr = self.v & 0x3FFF;

        let result = match addr {
            0x0000..=0x1FFF => {
                let result = self.internal_data_buff;
                self.internal_data_buff = self.chr_rom[addr as usize];
                result
            }
            0x2000..=0x3EFF => {
                let result = self.internal_data_buff;
                let index = self.mirror_vram_addr(addr) as usize;
                self.internal_data_buff = self.vram[index];
                result
            }
            0x3F00..=0x3FFF => {
                // Palette reads also refill the delayed read buffer
                // from the nametable address underneath the palette.
                let index = self.mirror_vram_addr(addr - 0x1000) as usize;
                self.internal_data_buff = self.vram[index];

                let value = self.palettes[Self::palette_index(addr)];
                if self.mask.contains(PpuMask::GRAYSCALE) {
                    value & 0x30
                } else {
                    value & 0x3F
                }
            }
            _ => unreachable!(),
        };

        self.increment_vram_addr();
        result
    }

    pub fn write_data(&mut self, value: u8) {
        let addr = self.v & 0x3FFF;

        match addr {
            0x0000..=0x1FFF => {
                if self.chr_is_ram {
                    self.chr_rom[addr as usize] = value;
                }
            }
            0x2000..=0x3EFF => {
                let index = self.mirror_vram_addr(addr) as usize;
                self.vram[index] = value;
            }
            0x3F00..=0x3FFF => {
                self.palettes[Self::palette_index(addr)] = value & 0x3F;
            }
            _ => unreachable!(),
        }

        self.increment_vram_addr();
    }

    pub fn mirror_vram_addr(&self, addr: u16) -> u16 {
        let index = (addr - 0x2000) & 0x0FFF;
        let table = index / 0x400;
        let offset = index & 0x03FF;

        let physical_table = match self.mirroring {
            Mirroring::Vertical => table & 1,
            Mirroring::Horizontal => table >> 1,
            Mirroring::FourScreen => table,
        };

        physical_table * 0x400 + offset
    }

    fn palette_index(addr: u16) -> usize {
        let mut index = (addr & 0x001F) as usize;

        if matches!(index, 0x10 | 0x14 | 0x18 | 0x1C) {
            index -= 0x10;
        }

        index
    }

    pub fn tick(&mut self) {
        self.cycles += 1;
        if self.scanline == 261 && self.cycles == 1 {
            self.status.remove(
                PpuStatus::VBLANK_FLAG
                    | PpuStatus::SRITE_0_FLAG
                    | PpuStatus::SPRITE_OVERFLOW_FLAG,
            );
            self.sprite_zero_dot = None;
        }

        if self.scanline < 240 {
            if self.cycles == 1 {
                let y = self.scanline as usize;

                let (pixels, hit_dot) =
                    crate::render::render_scanline(self, y, self.line_v);

                let start = y * 256;
                self.frame_pixels[start..start + 256]
                    .copy_from_slice(&pixels);

                self.sprite_zero_dot = hit_dot;
            }

            if self.sprite_zero_dot == Some(self.cycles) {
                self.status.insert(PpuStatus::SRITE_0_FLAG);
            }
        }

        if self.rendering_scanline() {
            if self.cycles == 321 {
                self.line_v = self.v;
            }

            if self.rendering_enabled() {
                let fetch_dot = (1..=256).contains(&self.cycles)
                    || (321..=336).contains(&self.cycles);

                if fetch_dot && self.cycles % 8 == 0 {
                    self.increment_x();
                }

                if self.cycles == 256 {
                    self.increment_y();
                }

                if self.cycles == 257 {
                    self.v = (self.v & !0x041F) | (self.t & 0x041F);
                }

                if self.scanline == 261
                    && (280..=304).contains(&self.cycles)
                {
                    self.v = (self.v & !0x7BE0) | (self.t & 0x7BE0);
                }
            }
        }

        if self.scanline == 241 && self.cycles == 1 {
            self.status.set_vblank_status(true);
            self.frame_ready = true;

            if self.ctrl.generate_vblank_nmi() {
                self.nmi_interrupt = Some(());
            }
        }

        if self.cycles >= 341 {
            self.cycles = 0;
            self.scanline += 1;

            if self.scanline >= 262 {
                self.scanline = 0;
            }
        }
    }


    fn rendering_enabled(&self) -> bool {
        self.mask.intersects(
            PpuMask::ENABLE_BACKGROUND | PpuMask::ENABLE_SPRITES,
        )
    }

    fn rendering_scanline(&self) -> bool {
        self.scanline < 240 || self.scanline == 261
    }

    fn increment_x(&mut self) {
        if self.v & 0x001F == 31 {
            self.v &= !0x001F;
            self.v ^= 0x0400;
        } else {
            self.v += 1;
        }
    }

    fn increment_y(&mut self) {
        if self.v & 0x7000 != 0x7000 {
            self.v += 0x1000;
            return;
        }

        self.v &= !0x7000;
        let mut coarse_y = (self.v & 0x03E0) >> 5;

        match coarse_y {
            29 => {
                coarse_y = 0;
                self.v ^= 0x0800;
            }
            31 => coarse_y = 0,
            _ => coarse_y += 1,
        }

        self.v = (self.v & !0x03E0) | (coarse_y << 5);
    }
}
