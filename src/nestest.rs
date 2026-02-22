use bus::Bus;
use rom::Rom;
use cpu::Cpu;
use crate::{bus, cpu, rom, trace::trace};

pub fn nestest() {
    let bytes: Vec<u8> = std::fs::read("nestest.nes").unwrap();
    let rom = Rom::new(&bytes).unwrap();

    let bus = Bus::new(rom, |_,_| {});
    let mut cpu = Cpu::new(bus);
    cpu.reset();
    cpu.program_counter = 0xC000;

    cpu.run_with_callback(move |cpu| {
        println!("{}", trace(cpu));
    });
}