pub fn get_rom_infos(raw: Vec<u8>) {
    let number = raw[7] & 0b1111_0000 | raw[6] >> 4;
    println!("Mapper number: {}", number);
    let prg_rom_size = raw[4];
    println!("PRG ROM size (16 KB units): {}", prg_rom_size);
    let chr_rom_size = raw[5];
    if chr_rom_size == 0 {
        println!("ROM uses CHR RAM");
    } else {
        println!("CHR ROM size (8 KB units): {}", chr_rom_size);
    }
}
