use crate::frame::Frame;
use crate::ppu::{Ppu, PpuController, PpuMask};
use crate::rom::Mirroring;

pub struct Rect {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

pub fn bg_palette(ppu: &Ppu, attribute_table: &[u8], tile_column: usize, tile_row: usize) -> [u8; 4] {
    let attr_tble_idx = tile_row / 4 * 8 + tile_column / 4;
    let attr_byte = attribute_table[attr_tble_idx];

    let palet_idx = match(tile_column % 4 / 2, tile_row % 4 / 2) {
        (0, 0) => attr_byte & 0b11,
        (0, 1) => (attr_byte >> 4) & 0b11,
        (1, 0) => (attr_byte >> 2) & 0b11,
        (1, 1) => (attr_byte >> 6) & 0b11,
        (_, _) => panic!("Cannot be"),
    };

    let palette_start: usize = 1 + (palet_idx as usize) * 4;
    [ppu.palettes[0], ppu.palettes[palette_start], ppu.palettes[palette_start + 1], ppu.palettes[palette_start +2]]
}

pub fn sprite_palette(ppu: &Ppu, palette_idx: u8) -> [u8; 4] {
    let start = 0x11 + (palette_idx * 4) as usize;
    [
        0,
        ppu.palettes[start],
        ppu.palettes[start + 1],
        ppu.palettes[start + 2],
    ]
}

pub fn render_name_table(
    ppu: &Ppu,
    frame: &mut Frame,
    name_table: &[u8],
    view_port: Rect,
    shift_x: isize,
    shift_y: isize
) {
    let bank: u16 = if ppu.ctrl.contains(PpuController::BG_PATTERN) { 0x1000 } else { 0 };
    let attribute_table = &name_table[0x3C0..0x400];

    for i in 0..0x3C0 {
        let tile_column = i % 32;
        let tile_row = i / 32;
        let tile_idx = name_table[i] as u16;
        let tile = &ppu.chr_rom[(bank + tile_idx * 16) as usize..=(bank + tile_idx * 16 + 15) as usize];
        let palette = bg_palette(ppu, attribute_table, tile_column, tile_row);

        for y in 0..8 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..8).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper >>= 1;
                lower >>= 1;

                let rgb = match value {
                    0 => frame.system_palette[ppu.palettes[0] as usize],
                    1 => frame.system_palette[palette[1] as usize],
                    2 => frame.system_palette[palette[2] as usize],
                    3 => frame.system_palette[palette[3] as usize],
                    _ => unreachable!(),
                };
                let pixel_x = tile_column * 8 + x;
                let pixel_y = tile_row * 8 + y;

                if pixel_x >= view_port.x1 && pixel_x < view_port.x2 && pixel_y >= view_port.y1 && pixel_y < view_port.y2 {
                    frame.set_pixel((shift_x + pixel_x as isize) as usize, (shift_y + pixel_y as isize) as usize, rgb);
                }
            }
        }
    }
}

pub fn render(ppu: &Ppu, frame: &mut Frame) {
    let scroll_x = ppu.scroll.scroll_x as usize;
    let scroll_y = ppu.scroll.scroll_y as usize;

    let base_nt = (ppu.ctrl.get_bg_bank() - 0x2000) as usize / 0x400;
    let mut nt_y = base_nt >> 1;
    let mut coarse_y = scroll_y >> 3;
    let mut fine_y = scroll_y & 7;

    let bank: usize = if ppu.ctrl.contains(PpuController::BG_PATTERN) { 0x1000 } else { 0 };

    let mut bg_opaque = vec![false; 256 * 240];
    let mut sprite_claimed = vec![false; 256 * 240];

    for screen_y in 0..240 {
        for screen_x in 0..256 {
            let source_x = scroll_x + screen_x;
            let coarse_x = (source_x >> 3) & 31;
            let fine_x = source_x & 7;

            let nt_x = (base_nt & 1) ^ (source_x >> 8);

            let logical_nt = (nt_y << 1) | nt_x;

            let (logical_nt, coarse_x, fine_x, coarse_y, fine_y) =
                if screen_y < 32 && scroll_x != 0{
                    (0usize, screen_x / 8, screen_x & 7, screen_y / 8, screen_y & 7)
                } else {
                    (logical_nt, coarse_x, fine_x, coarse_y, fine_y)
                };
            
            let physical_nt = match ppu.mirroring {
                Mirroring::Vertical => logical_nt & 1,
                Mirroring::Horizontal => logical_nt >> 1,
                Mirroring::FourScreen => {
                    panic!("Four-screen mirroring is not supported")
                },
            };

            let start = physical_nt * 0x400;
            let name_table = &ppu.vram[start..start + 0x400];

            let tile_idx = name_table[coarse_y * 32 + coarse_x] as usize;
            let tile_start = bank + tile_idx * 16;
            let bg_visible = ppu.mask.contains(PpuMask::ENABLE_BACKGROUND)
                && (screen_x >= 8 || ppu.mask.contains(PpuMask::SHOW_BG_LEFT));

            let bit = 7 - fine_x;
            let low = (ppu.chr_rom[tile_start + fine_y] >> bit) & 1;
            let high = (ppu.chr_rom[tile_start + fine_y + 8] >> bit) & 1;
            let value = if bg_visible { ((high << 1) | low) as usize } else { 0 };
            bg_opaque[screen_y * 256 + screen_x] = value != 0;

            let palette = bg_palette(
                ppu,
                &name_table[0x3C0..0x400],
                coarse_x,
                coarse_y,
            );

            let rgb = frame.system_palette[palette[value] as usize];
            frame.set_pixel(screen_x, screen_y, rgb);
        }

        fine_y += 1;

        if fine_y == 8 {
            fine_y = 0;

            match coarse_y {
                29 => {
                    coarse_y = 0;
                    nt_y ^= 1;
                }
                31 => {
                    coarse_y = 0;
                }
                _ => coarse_y += 1,
            }
        }
    }

    // Rendering sprites
    for k in (0..ppu.oam.len()).step_by(4) {
        let tile_idx = ppu.oam[k + 1] as u16;
        let tile_x = ppu.oam[k + 3] as usize;
        let tile_y = ppu.oam[k] as usize + 1;

        let is_size_16 = ppu.ctrl.contains(PpuController::SPRITE_SIZE);

        let flip_vertical = ppu.oam[k + 2] >> 7 & 1 == 1;

        let flip_horizontal = ppu.oam[k + 2] >> 6 & 1 == 1;

        let palette_idx = ppu.oam[k + 2] & 0b11;
        let sprite_palette = sprite_palette(ppu, palette_idx);

        let (bank, base_tile, height): (u16, u16, usize) = if is_size_16 {
            let bank = if tile_idx & 1 != 0 { 0x1000 } else { 0 };
            (bank, tile_idx & 0xFE, 16)
        } else {
            (ppu.ctrl.sprite_pattern_addr(), tile_idx, 8)
        };

        for row in 0..(height / 8) {
            let sub_tile_idx = base_tile + row as u16;
        
            let tile_start = (bank + sub_tile_idx * 16) as usize;
            let tile = &ppu.chr_rom[tile_start..tile_start + 16];
        
            for y in 0..8 {
                let mut upper = tile[y];
                let mut lower = tile[y + 8];
            
                for x in (0..8).rev() {
                    let value = (lower & 1) << 1 | (upper & 1);
                
                    upper >>= 1;
                    lower >>= 1;
                
                    let rgb = match value {
                        0 => continue,
                        1 => frame.system_palette[sprite_palette[1] as usize],
                        2 => frame.system_palette[sprite_palette[2] as usize],
                        3 => frame.system_palette[sprite_palette[3] as usize],
                        _ => unreachable!(),
                    };
                
                    let out_x = if flip_horizontal { tile_x + 7 - x } else { tile_x + x };
                    let out_y = if flip_vertical { tile_y + (height - 1) - (row * 8 + y) } else { tile_y + row * 8 + y };

                    if out_x >= 256 || out_y >= 240 {
                        continue;
                    }

                    if !ppu.mask.contains(PpuMask::ENABLE_SPRITES)
                        || (out_x < 8 && !ppu.mask.contains(PpuMask::SHOW_SPRITES_LEFT))
                    {
                        continue;
                    }

                    let index = out_y * 256 + out_x;

                    if sprite_claimed[index] {
                        continue;
                    }
                    sprite_claimed[index] = true;

                    let behind_background = ppu.oam[k + 2] & 0x20 != 0;

                    if !behind_background || !bg_opaque[index] {
                        frame.set_pixel(out_x, out_y, rgb);
                    }
                }
            }
        }
    }
}
