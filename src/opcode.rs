use crate::cpu::AddressingMode;
use std::collections::HashMap;
use lazy_static::lazy_static;

pub struct OpCode {
    pub code: u8,
    pub nmemonic: &'static str,
    pub len: u8,
    pub cycles: u8,
    pub mode: AddressingMode,
}

impl OpCode {
    pub fn new(code: u8, nmemonic: &'static str, len: u8, cycles: u8, mode: AddressingMode) -> Self {
        OpCode {
            code,
            nmemonic,
            len,
            cycles,
            mode,
        }
    }
}

lazy_static! {
    pub static ref CPU_OP_CODES: Vec<OpCode> = vec![
        OpCode::new(0x90, "BCC", 2, 2 /*+1 if succeeds, 2 if new page */, AddressingMode::NoneAdressing),
        OpCode::new(0xB0, "BCC", 2, 2 /*+1 if succeeds, 2 if new page */, AddressingMode::NoneAdressing),
        OpCode::new(0xF0, "BEQ", 2, 2 /*+1 if succeeds, 2 if new page */, AddressingMode::NoneAdressing),
        OpCode::new(0x30, "BMI", 2, 2 /*+1 if succeeds, 2 if new page */, AddressingMode::NoneAdressing),
        OpCode::new(0xD0, "BNE", 2, 2 /*+1 if succeeds, 2 if new page */, AddressingMode::NoneAdressing),
        OpCode::new(0x10, "BPL", 2, 2 /*+1 if succeeds, 2 if new page */, AddressingMode::NoneAdressing),
        OpCode::new(0x00, "BRK", 1, 7, AddressingMode::NoneAdressing),
        OpCode::new(0x50, "BVC", 2, 2 /*+1 if succeeds, 2 if new page */, AddressingMode::NoneAdressing),
        OpCode::new(0x70, "BVS", 2, 2 /*+1 if succeeds, 2 if new page */, AddressingMode::NoneAdressing),
        OpCode::new(0x18, "CLC", 1, 2, AddressingMode::NoneAdressing),
        OpCode::new(0xD8, "CLD", 1, 2, AddressingMode::NoneAdressing),
        OpCode::new(0x58, "CLI", 1, 2, AddressingMode::NoneAdressing),
        OpCode::new(0xB8, "CLV", 1, 2, AddressingMode::NoneAdressing),
        OpCode::new(0xE8, "INX", 1, 2, AddressingMode::NoneAdressing),
        OpCode::new(0xC8, "INY", 1, 2, AddressingMode::NoneAdressing),
        OpCode::new(0xAA, "TAX", 1, 2, AddressingMode::NoneAdressing),
        OpCode::new(0xA8, "TAY", 1, 2, AddressingMode::NoneAdressing),
        OpCode::new(0x8A, "TXA", 1, 2, AddressingMode::NoneAdressing),
        OpCode::new(0x9A, "TXS", 1, 2, AddressingMode::NoneAdressing),
        OpCode::new(0xBA, "TSX", 1, 2, AddressingMode::NoneAdressing),
        OpCode::new(0x98, "TYA", 1, 2, AddressingMode::NoneAdressing),
        OpCode::new(0xEA, "NOP", 1, 2, AddressingMode::NoneAdressing),
        //OpCode::new(0x40, "RTI", 1, 6, AddressingMode::NoneAdressing),
        OpCode::new(0x60, "RTS", 1, 6, AddressingMode::NoneAdressing),
        OpCode::new(0x38, "SEC", 1, 2, AddressingMode::NoneAdressing),
        OpCode::new(0xF8, "SED", 1, 2, AddressingMode::NoneAdressing),
        OpCode::new(0x78, "SEI", 1, 2, AddressingMode::NoneAdressing),
        /*AND */
        OpCode::new(0x29, "AND", 2, 2, AddressingMode::NoneAdressing),
        OpCode::new(0x25, "AND", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x35, "AND", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x2D, "AND", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x3D, "AND", 3, 4, AddressingMode::AbsoluteX),
        OpCode::new(0x39, "AND", 3, 4, AddressingMode::AbsoluteY),
        OpCode::new(0x21, "AND", 2, 6, AddressingMode::IndirectX),
        OpCode::new(0x31, "AND", 2, 5, AddressingMode::IndirectY),
        /*ASL */
        OpCode::new(0x0A, "ASL", 1, 2, AddressingMode::Accumulator),
        OpCode::new(0x06, "ASL", 2, 5, AddressingMode::ZeroPage),
        OpCode::new(0x16, "ASL", 2, 6, AddressingMode::ZeroPageX),
        OpCode::new(0x0E, "ASL", 3, 6, AddressingMode::Absolute),
        OpCode::new(0x1E, "ASL", 3, 7, AddressingMode::AbsoluteX),
        /*BIT */
        OpCode::new(0x24, "BIT", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x2C, "BIT", 3, 4, AddressingMode::Absolute),
        /*CMP */
        OpCode::new(0xC9, "CMP", 2, 2, AddressingMode::NoneAdressing),
        OpCode::new(0xC5, "CMP", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xD5, "CMP", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0xCD, "CMP", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xDD, "CMP", 3, 4 /*+ 1 if page crossed */, AddressingMode::AbsoluteX),
        OpCode::new(0xD9, "CMP", 3, 4 /*+1 if page crossed */, AddressingMode::AbsoluteY),
        OpCode::new(0xC1, "CMP", 2, 6, AddressingMode::IndirectX),
        OpCode::new(0xD1, "CMP", 2, 5 /*+1 if page crossed */, AddressingMode::IndirectY),
        /*CPX */
        OpCode::new(0xE0, "CPX", 2, 2, AddressingMode::NoneAdressing),
        OpCode::new(0xE4, "CPX", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xEC, "CPX", 3, 4, AddressingMode::Absolute),
        /*CPY */
        OpCode::new(0xC0, "CPY", 2, 2, AddressingMode::NoneAdressing),
        OpCode::new(0xC4, "CPY", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xCC, "CPY", 3, 4, AddressingMode::Absolute),
        /*JMP */
        OpCode::new(0x4C, "JMP", 3, 3, AddressingMode::Absolute),
        OpCode::new(0x6C, "JMP", 3, 5, AddressingMode::Indirect),
        /*LDA */
        OpCode::new(0xA9, "LDA", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xA5, "LDA", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xB5, "LDA", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0xAD, "LDA", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xBD, "LDA", 3, 4, AddressingMode::AbsoluteX),
        OpCode::new(0xB9, "LDA", 3, 4, AddressingMode::AbsoluteY),
        OpCode::new(0xA1, "LDA", 2, 6, AddressingMode::IndirectX),
        OpCode::new(0xB1, "LDA", 2, 5, AddressingMode::IndirectY),
        /*LDX */
        OpCode::new(0xA2, "LDX", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xA6, "LDX", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xB6, "LDX", 2, 4, AddressingMode::ZeroPageY),
        OpCode::new(0xAE, "LDX", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xBE, "LDX", 3, 4/*+1 if crossed */, AddressingMode::AbsoluteY),
        /*LDY */
        OpCode::new(0xA0, "LDY", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xA4, "LDY", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xB4, "LDY", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0xAC, "LDY", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xBC, "LDY", 3, 4 /*+1 if crossed */, AddressingMode::AbsoluteX),
        /*STA */
        OpCode::new(0x85, "STA", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x95, "STA", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x8D, "STA", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x9D, "STA", 3, 5, AddressingMode::AbsoluteX),
        OpCode::new(0x99, "STA", 3, 5, AddressingMode::AbsoluteY),
        OpCode::new(0x81, "STA", 2, 6, AddressingMode::IndirectX),
        OpCode::new(0x91, "STA", 2, 6, AddressingMode::IndirectY),
        /*STX */
        OpCode::new(0x86, "STX", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x96, "STX", 2, 4, AddressingMode::ZeroPageY),
        OpCode::new(0x8E, "STX", 3, 4, AddressingMode::Absolute),
        /*STY */
        OpCode::new(0x84, "STY", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x94, "STY", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x8C, "STY", 3, 4, AddressingMode::Absolute),
    ];

    pub static ref OPCODES_MAP: HashMap<u8, &'static OpCode> = {
        let mut  map = HashMap::new();
        for cpucode in &*CPU_OP_CODES {
            map.insert(cpucode.code, cpucode);
        }
        map
    };
}
