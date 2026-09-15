use std::{time::{Duration, Instant}};

use crate::{joypad::JoyPad, ppu::Ppu, rom::Rom, apu::Apu, apu::CPU_FREQ, sound::SoundBackEnd};

const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1FFF;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;

const CPU_CYCLES_PER_FRAME: f64 = 341.0 * 262.0 / 3.0;

pub struct Bus<'call> {
    cpu_vram: [u8; 2048],
    rom: Rom,
    ppu: Ppu,
    pub joypad: JoyPad,
    pub apu: Apu,
    sound: SoundBackEnd,

    apu_cycle_toggle: bool,
    sample_counter: f64,

    cycles: usize,
    gameloop_callback: Box<dyn FnMut(&Ppu, &mut JoyPad) + 'call>,
    next_frame: Instant,
}

impl<'a> Bus<'a> {
    pub fn new<'call, F>(rom: Rom, gameloop_callback: F) -> Bus<'call> where F: FnMut(&Ppu, &mut JoyPad) + 'call {
        let chr_rom = rom.chr_rom.clone();
        let ppu = Ppu::new(chr_rom, rom.screen_miroring);
        let joypad = JoyPad::new();
        let apu = Apu::new();
        let sound = SoundBackEnd::new();
        Bus {
            cpu_vram: [0; 2048],
            rom,
            ppu,
            cycles: 0,
            gameloop_callback: Box::from(gameloop_callback),
            joypad,
            apu,
            sound,
            apu_cycle_toggle: false,
            sample_counter: 0.0,
            next_frame: Instant::now(),
        }
    }
    pub fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b11111111111;
                self.cpu_vram[mirror_down_addr as usize]
            },
            0x2000 | 0x2001 | 0x2003 | 0x2005 | 0x4014 => {
                //panic!("Attempt to read from write-only PPU address {:x}", addr);
                0xFF
            },
            0x2002 => self.ppu.read_status(),
            0x2007 => self.ppu.read_data(),
            0x2008..=PPU_REGISTERS_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00100000_00000111;
                self.mem_read(mirror_down_addr)
            },
            0x4015 => self.apu.read_status(),
            0x4016 => self.joypad.read(),
            0x4017 => 0x40, // 2nd joypad not implemented yet 
            0x8000..=0xFFFF => self.read_prg_rom(addr),
            _ => {
                //println!("Ignoring mem access at {:x}", addr);
                0x40
            }

        }
    }

    pub fn mem_write(&mut self, value: u8, addr: u16) {
        match addr {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b11111111111;
                self.cpu_vram[mirror_down_addr as usize] = value;
            },
            0x2000 => self.ppu.write_to_ctrl(value),
            0x2001 => self.ppu.write_to_mask(value),
            0x2003 => self.ppu.write_oam_addr(value),
            0x2004 => self.ppu.write_oam_data(value),
            0x2005 => self.ppu.write_scroll(value),
            0x2006 => self.ppu.write_addr_registers(value),
            0x2007 => self.ppu.write_data(value),
            0x2008..=PPU_REGISTERS_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00100000_00000111;
                self.mem_write(value, mirror_down_addr);
            },
            0x4000..=0x4003 => self.apu.pulse_1.write_pulse(addr, &value),
            0x4004..=0x4007 => self.apu.pulse_2.write_pulse(addr, &value),
            0x4008..=0x400B => self.apu.triangle.write_triangle(addr, &value),
            0x4014 => {
                let page_start = (value as u16) << 8;

                for i in 0..=255u16 {
                    let data = self.mem_read(page_start + i);
                    self.ppu.write_oam_data(data);
                }

                let dma_cycles = 513 + (self.cycles & 1);
                self.tick(dma_cycles);
            },
            0x4015 => self.apu.write_status(value),
            0x4016 => self.joypad.write(value),
            0x4017 => self.apu.write_frame_counter(value),
            0x8000..=0xFFFF => { /*panic!("Attempt to write to Cartridge ROM space: {:x}", addr) */ }
            _ => {
                //println!("Ignoring memory writing for {}", addr);
            }
        }
    }

    pub fn read_prg_rom(&mut self, mut addr: u16) -> u8 {
        addr -= 0x8000;
        if self.rom.prg_rom.len() == 0x4000 && addr >= 0x4000 {
            addr %= 0x4000;
        }
        self.rom.prg_rom[addr as usize]
    }

    pub fn poll_nmi(&mut self) -> bool {
        self.ppu.nmi_interrupt.take().is_some()    
    }

    pub fn tick(&mut self, cycles: usize) {
        let frame_time: Duration = Duration::from_secs_f64(CPU_CYCLES_PER_FRAME / CPU_FREQ);

        self.cycles += cycles;

        (0..(cycles * 3)).for_each(|_| self.ppu.tick());

        if std::mem::take(&mut self.ppu.frame_ready) {
            let now = Instant::now();

            if self.next_frame > now {
                std::thread::sleep(self.next_frame - now);
            }

            self.next_frame += frame_time;

            (self.gameloop_callback)(&self.ppu, &mut self.joypad);
        }

        let sample_rate = self.sound.sample_rate;
        for _ in 0..cycles {
            self.apu.clock_frame_counter();

            // Pulse timers run at half the CPU frequency (every other CPU cycle).
            self.apu_cycle_toggle = !self.apu_cycle_toggle;
            if self.apu_cycle_toggle {
                self.apu.tick();
            }

            // Emit a mixed sample at the host audio sample rate.
            self.sample_counter += sample_rate / CPU_FREQ;
            if self.sample_counter >= 1.0 {
                self.sample_counter -= 1.0;
                self.sound.push(self.apu.sample());
            }
        }
    }
}