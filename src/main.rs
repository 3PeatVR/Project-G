pub mod bus;
pub mod rom;
pub mod cpu;
pub mod opcode;
pub mod trace;
pub mod nestest;
pub mod ppu;
pub mod frame;
pub mod render_tiles;
pub mod render;
pub mod joypad;
pub mod apu;
pub mod sound;
pub mod config;
pub mod rom_menu;

use std::collections::HashMap;

use clap::{Parser, Subcommand};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

use bus::Bus;
use cpu::Cpu;
use frame::Frame;
use ppu::Ppu;
use rom::Rom;

use crate::config::Config;
use crate::joypad::JoyPad;

#[derive(Parser)]
#[command(name = "project-G")]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    Test,
    RenderTiles,
    Play,
    LoadRom,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Test => { nestest::nestest() },
        Commands::RenderTiles => { render_tiles::render_tiles(); },
        Commands::Play => {
            let config = Config::load_config();
            let rom_path = config.loaded_rom.as_ref().expect("No rom is placed in the ./roms/ folder");
            let bytes = std::fs::read(rom_path).expect("Rom could not be loaded");
            let rom = Rom::new(&bytes).unwrap();

            if rom.mapper > 0 {
                panic!("Mapper not supported by Project G");
            }

            let sdl_context = sdl2::init().unwrap();
            let video = sdl_context.video().unwrap();
            let window = video
                .window("NES Emulator", 256 * 3, 240 * 3)
                .position_centered()
                .build()
                .unwrap();

            let mut canvas = window.into_canvas().build().unwrap();
            let mut event_pump = sdl_context.event_pump().unwrap();
            canvas.set_scale(3.0, 3.0).unwrap();

            let creator = canvas.texture_creator();
            let texture = creator
                .create_texture_target(PixelFormatEnum::RGB24, 256, 240)
                .unwrap();

            let mut texture: sdl2::render::Texture<'static> = unsafe { std::mem::transmute(texture) };

            let mut frame = Frame::new();

            let mut key_map = HashMap::new();
            key_map.insert(config.controls.down, joypad::InputsButtons::DOWN);
            key_map.insert(config.controls.up, joypad::InputsButtons::UP);
            key_map.insert(config.controls.left, joypad::InputsButtons::LEFT);
            key_map.insert(config.controls.right, joypad::InputsButtons::RIGHT);
            key_map.insert(config.controls.select, joypad::InputsButtons::SELECT);
            key_map.insert(config.controls.start, joypad::InputsButtons::START);
            key_map.insert(config.controls.a, joypad::InputsButtons::A);
            key_map.insert(config.controls.b, joypad::InputsButtons::B);

            // Show the window immediately so the user can interact before the first NMI fires.
            canvas.present();

            let bus = Bus::new(rom, move |ppu: &Ppu, _joypad: &mut JoyPad| {
                render::render(ppu, &mut frame);
                texture.update(None, &frame.data, 256 * 3).unwrap();
                canvas.copy(&texture, None, None).unwrap();
                canvas.present();
            });

            let mut cpu = Cpu::new(bus);
            cpu.reset();
            cpu.run_with_callback(move |cpu| {
                for event in event_pump.poll_iter() {
                    match event {
                        Event::Quit { .. }
                        | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                            std::process::exit(0);
                        },
                        Event::KeyDown { keycode, .. } => {
                            if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                                cpu.bus.joypad.set_button_pressed_status(key, true);
                            }
                        },
                        Event::KeyUp { keycode, .. } => {
                            if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                                cpu.bus.joypad.set_button_pressed_status(key, false);
                            }
                        },
                        _ => {}
                    }
                }
            });
        },
        Commands::LoadRom => {
            let _ = rom_menu::load_rom_menu();
        }
    }
}
