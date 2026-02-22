use crate::frame::Frame;
use crate::ppu::{Ppu, PpuController};
use crate::rom::Mirroring;

pub struct Rect {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl Rect {
    fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Rect { x1, y1, x2, y2 }
    }
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
    
    let (main_table, second_table) = match(&ppu.mirroring, ppu.ctrl.get_bg_bank()) {
        (Mirroring::Vertical, 0x2000) | (Mirroring::Vertical, 0x2800) | (Mirroring::Horizontal, 0x2000) | (Mirroring::Horizontal, 0x2400) => {
            (&ppu.vram[0..0x400], &ppu.vram[0x400..0x800])
        }
        (Mirroring::Vertical, 0x2400) | (Mirroring::Vertical, 0x2C00) | (Mirroring::Horizontal, 0x2800) | (Mirroring::Horizontal, 0x2C00) => {
            (&ppu.vram[0x400..0x800], &ppu.vram[0..0x400])
        }
        (_, _) => panic!("not supported mirroring type")
    };

    render_name_table(ppu, frame, main_table, Rect::new(scroll_x, scroll_y, 256, 240), -(scroll_x as isize), -(scroll_y as isize));
    render_name_table(ppu, frame, second_table, Rect::new(0, 0, scroll_x, 240), (256 - scroll_x) as isize, 0);
    render_name_table(ppu, frame, second_table, Rect::new(0, 0, 256, scroll_y), 0, (240 - scroll_y) as isize);

    // Rendering sprites
    for k in (0..ppu.oam.len()).step_by(4).rev() {
        let tile_idx = ppu.oam[k + 1] as u16;
        let tile_x = ppu.oam[k + 3] as usize;
        let tile_y = ppu.oam[k] as usize;

        let flip_vertical = if ppu.oam[k + 2] >> 7 & 1 == 1 {
            true
        } else {
            false
        };

        let flip_horizontal = if ppu.oam[k + 2] >> 6 & 1 == 1 {
            true
        } else {
            false
        };

        let palette_idx = ppu.oam[k + 2] & 0b11;
        let sprite_palette = sprite_palette(ppu, palette_idx);

        let bank: u16 = ppu.ctrl.sprite_pattern_addr();

        let tile = &ppu.chr_rom[(bank + tile_idx * 16) as usize..=(bank + tile_idx * 16 + 15) as usize];

        for y in 0..8 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];
            'tag: for x in (0..8).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper >>= 1;
                lower >>= 1;
                let rgb = match value {
                    0 => continue 'tag,
                    1 => frame.system_palette[sprite_palette[1] as usize],
                    2 => frame.system_palette[sprite_palette[2] as usize],
                    3 => frame.system_palette[sprite_palette[3] as usize],
                    _ => panic!("cannot be"),
                };
                match (flip_horizontal, flip_vertical) {
                    (false, false) => frame.set_pixel(tile_x + x, tile_y + y, rgb),
                    (false, true) => frame.set_pixel(tile_x + x, tile_y + 7 - y, rgb),
                    (true, false) => frame.set_pixel(tile_x + 7 - x, tile_y + y, rgb),
                    (true, true) => frame.set_pixel(tile_x + 7 - x, tile_y + 7 - y, rgb),
                } 
            }
        }
    }
}
