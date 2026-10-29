use crate::frame::Frame;
use crate::ppu::{Ppu, PpuController, PpuMask};

fn pattern_pixel(
    ppu: &Ppu,
    tile_start: usize,
    x: usize,
    y: usize,
) -> u8 {
    let bit = 7 - x;
    let low = (ppu.rom.borrow().mapper.get_chr_rom((tile_start + y) as u16) >> bit) & 1;
    let high = (ppu.rom.borrow().mapper.get_chr_rom((tile_start + y + 8) as u16) >> bit) & 1;

    (high << 1) | low
}

pub(crate) fn render_scanline(
    ppu: &Ppu,
    screen_y: usize,
    line_v: u16,
) -> ([u8; 256], Option<usize>) {
    let mut pixels = [ppu.palettes[0]; 256];
    let mut bg_opaque = [false; 256];
    let mut sprite_claimed = [false; 256];
    let mut hit_dot = None;

    let bg_enabled = ppu.mask.contains(PpuMask::ENABLE_BACKGROUND);
    let sprites_enabled = ppu.mask.contains(PpuMask::ENABLE_SPRITES);

    // Forced blank can output the palette entry addressed by v.
    if !bg_enabled && !sprites_enabled && ppu.v & 0x3F00 == 0x3F00 {
        let mut index = (ppu.v & 0x1F) as usize;
        if matches!(index, 0x10 | 0x14 | 0x18 | 0x1C) {
            index -= 0x10;
        }
        pixels.fill(ppu.palettes[index]);
    }

    if bg_enabled {
        let bank = if ppu.ctrl.contains(PpuController::BG_PATTERN) {
            0x1000
        } else {
            0
        };

        let base_nt = ((line_v >> 10) & 3) as usize;
        let base_coarse_x = (line_v & 31) as usize;
        let coarse_y = ((line_v >> 5) & 31) as usize;
        let fine_y = ((line_v >> 12) & 7) as usize;

        for screen_x in 0..256 {
            if screen_x < 8 && !ppu.mask.contains(PpuMask::SHOW_BG_LEFT) {
                continue;
            }

            let source_x = ppu.fine_x as usize + screen_x;
            let tile_x = base_coarse_x + (source_x >> 3);

            let coarse_x = tile_x & 31;
            let fine_x = source_x & 7;
            let logical_nt = base_nt ^ ((tile_x >> 5) & 1);

            let nt_addr = 0x2000 + logical_nt as u16 * 0x400;
            let start = ppu.mirror_vram_addr(nt_addr) as usize;

            let tile_idx = ppu.vram[
                start + coarse_y * 32 + coarse_x
            ] as usize;

            let value = pattern_pixel(
                ppu,
                bank + tile_idx * 16,
                fine_x,
                fine_y,
            );

            if value == 0 {
                continue;
            }

            let attribute = ppu.vram[
                start + 0x3C0 + (coarse_y / 4) * 8 + coarse_x / 4
            ];

            let shift = ((coarse_y & 2) << 1) | (coarse_x & 2);
            let palette_idx = ((attribute >> shift) & 3) as usize;

            pixels[screen_x] =
                ppu.palettes[palette_idx * 4 + value as usize];
            bg_opaque[screen_x] = true;
        }
    }

    if sprites_enabled {
        let size_16 = ppu.ctrl.contains(PpuController::SPRITE_SIZE);
        let height = if size_16 { 16 } else { 8 };
        let mut selected = 0;

        for sprite in 0..64 {
            let k = sprite * 4;
            let top = ppu.oam[k] as usize + 1;

            if screen_y < top || screen_y >= top + height {
                continue;
            }

            if selected == 8 {
                break;
            }
            selected += 1;

            let tile_idx = ppu.oam[k + 1] as usize;
            let attributes = ppu.oam[k + 2];
            let left = ppu.oam[k + 3] as usize;

            let mut source_y = screen_y - top;
            if attributes & 0x80 != 0 {
                source_y = height - 1 - source_y;
            }

            let (bank, tile) = if size_16 {
                (
                    (tile_idx & 1) * 0x1000,
                    (tile_idx & 0xFE) + source_y / 8,
                )
            } else {
                (ppu.ctrl.sprite_pattern_addr() as usize, tile_idx)
            };

            let tile_start = bank + tile * 16;
            let palette_idx = (attributes & 3) as usize;
            let behind_background = attributes & 0x20 != 0;

            for dx in 0..8 {
                let screen_x = left + dx;

                if screen_x >= 256 {
                    continue;
                }

                if screen_x < 8
                    && !ppu.mask.contains(PpuMask::SHOW_SPRITES_LEFT)
                {
                    continue;
                }

                let source_x = if attributes & 0x40 != 0 {
                    7 - dx
                } else {
                    dx
                };

                let value = pattern_pixel(
                    ppu,
                    tile_start,
                    source_x,
                    source_y & 7,
                );

                if value == 0 {
                    continue;
                }

                if sprite == 0
                    && screen_x != 255
                    && bg_opaque[screen_x]
                    && hit_dot.is_none()
                {
                    hit_dot = Some(screen_x + 2);
                }

                if sprite_claimed[screen_x] {
                    continue;
                }
                sprite_claimed[screen_x] = true;

                if !behind_background || !bg_opaque[screen_x] {
                    pixels[screen_x] = ppu.palettes[
                        0x10 + palette_idx * 4 + value as usize
                    ];
                }
            }
        }
    }

    let color_mask = if ppu.mask.contains(PpuMask::GRAYSCALE) {
        0x30
    } else {
        0x3F
    };

    for color in &mut pixels {
        *color &= color_mask;
    }

    (pixels, hit_dot)
}

pub fn render(ppu: &Ppu, frame: &mut Frame) {
    for y in 0..240 {
        for x in 0..256 {
            let color = ppu.frame_pixels[y * 256 + x] as usize;
            let rgb = frame.system_palette[color];
            frame.set_pixel(x, y, rgb);
        }
    }
}
