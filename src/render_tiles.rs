use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::fs;
use std::path::Path;

use crate::rom::Rom;
use crate::{frame::Frame};

const SCALE: u32 = 3;
const TILES_PER_ROW: usize = 16;
const TILE_SIZE: usize = 9;
const FRAME_WIDTH: usize = 256;

fn find_first_nes_rom(dir: &Path) -> Option<std::path::PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return None;
    };
    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_first_nes_rom(&path) {
                return Some(found);
            }
        } else if path.extension().map(|e| e.to_ascii_lowercase() == "nes").unwrap_or(false) {
            return Some(path);
        }
    }
    None
}


fn show_tile(chr_rom: &Vec<u8>, bank: usize, tile_n: usize) -> Frame {
    assert!(bank <= 1);

    let mut frame = Frame::new();
    let bank = (bank * 0x1000) as usize;

    let tile = &chr_rom[(bank + tile_n * 16)..=(bank + tile_n * 16 + 15)];

    for y in 0..8 {
        let mut upper = tile[y];
        let mut lower = tile[y + 8];
        for x in (0..8).rev() {
            let value = (1 & upper) << 1 | (1 & lower);
            upper >>= 1;
            lower >>= 1;
            let rgb = match value {
                0 => frame.system_palette[0x01],
                1 => frame.system_palette[0x22],
                2 => frame.system_palette[0x24],
                3 => frame.system_palette[0x30],
                _ => panic!("cannot be"),
            };
            frame.set_pixel(x, y, rgb);
        }
    }
    frame
}


pub fn render_tiles() {
    let tests_rom_dir = Path::new("./tests_rom/mojontwinsmulticart");

    let rom_path = match find_first_nes_rom(tests_rom_dir) {
        Some(p) => p,
        None => {
            println!("No ROM found in ./tests_rom");
            return;
        }
    };

    println!("Loading ROM: {}", rom_path.display());

    let raw = fs::read(&rom_path).expect("Failed to read ROM file");
    let rom = Rom::new(&raw).expect("Failed to parse ROM");

    // Each bank half is 0x1000 (4096) bytes = 256 tiles of 16 bytes each.
    // show_tile supports bank 0 and bank 1 only.
    let num_banks = (rom.chr_rom.len() / 0x1000).min(2);

    if num_banks == 0 {
        println!("ROM has no CHR ROM data to display");
        return;
    }

    let canvas_w = (TILES_PER_ROW * TILE_SIZE) as u32;
    let canvas_h = (num_banks * TILES_PER_ROW * TILE_SIZE) as u32;

    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    let window = video
        .window(
            &format!("Tile Viewer — {}", rom_path.file_name().unwrap().to_string_lossy()),
            canvas_w * SCALE,
            canvas_h * SCALE,
        )
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, canvas_w, canvas_h)
        .unwrap();

    texture
        .with_lock(None, |buf, pitch| {
            for bank in 0..num_banks {
                for tile_n in 0..256_usize {
                    let frame = show_tile(&rom.chr_rom, bank, tile_n);
                    let tile_col = tile_n % TILES_PER_ROW;
                    let tile_row = bank * TILES_PER_ROW + tile_n / TILES_PER_ROW;

                    for y in 0..TILE_SIZE {
                        for x in 0..TILE_SIZE {
                            let src = y * 3 * FRAME_WIDTH + x * 3;
                            let dst_x = tile_col * TILE_SIZE + x;
                            let dst_y = tile_row * TILE_SIZE + y;
                            let dst = dst_y * pitch + dst_x * 3;
                            buf[dst] = frame.data[src];
                            buf[dst + 1] = frame.data[src + 1];
                            buf[dst + 2] = frame.data[src + 2];
                        }
                    }
                }
            }
        })
        .unwrap();

    canvas.copy(&texture, None, None).unwrap();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }
    }
}
