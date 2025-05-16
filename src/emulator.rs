use std::time::{Duration, Instant};

/// Clock rate in Hz
pub const RATE: f32 = 2_000_000.0;
const PERIOD: f32 = 1.0 / RATE;
const KB: usize = 1024;
pub const MEM_SIZE: usize = KB * 64;

#[allow(dead_code)]
enum Mode<'a> {
    Normal {
        reader: Option<Box<dyn FnMut(u8) -> u8 + 'a>>,
        writer: Option<Box<dyn FnMut(u8, u8) + 'a>>,
    },
    Testing(bool),
}

pub struct Emulator<'a> {
    /// Stack pointer
    sp: u16,
    /// Program Counter
    pc: u16,
    /// Flag register in order S,Z,0,A,0,P,1,C where
    /// S: Sign flag
    /// Z: Zero Flag,
    /// 0: Unused, always 0,
    /// A: Auxillary Carry flag,
    /// 0: Unused, always 0,
    /// P: Parity Flag,
    /// 1: Unsed, always 1
    /// C: Carry flag
    flag: u8,
    /// Memory
    memory: [u8; MEM_SIZE],
    /// Registers in order B,C,D,E,H,L,A
    registers: [u8; 7],

    now: Instant,
    end_cycle: Duration,

    halt: bool,
    /// If true, the Interrupt flip-flop is enabled
    interrupt: bool,

    mode: Mode<'a>,
}

impl<'a> Emulator<'a> {
    pub fn new(program: &[u8]) -> Self {
        let mut memory = [0; MEM_SIZE];
        let len = program.len().min(MEM_SIZE);

        memory[..len].copy_from_slice(&program[..len]);

        Self::new_from_start(memory, 0x00)
    }

    pub fn new_from_start(program: [u8; MEM_SIZE], start: u16) -> Self {
        Self {
            pc: start,
            sp: 0xFFFF,
            flag: 2,
            memory: program,
            registers: [0; 7],
            now: Instant::now(),
            end_cycle: Duration::ZERO,
            halt: false,
            interrupt: false,
            mode: Mode::Normal {
                reader: None,
                writer: None,
            },
        }
    }

    #[allow(dead_code)]
    fn new_test(program: [u8; MEM_SIZE], start: u16) -> Self {
        let mut new = Self::new_from_start(program, start);
        new.mode = Mode::Testing(true);
        new
    }

    pub fn cycle(&mut self) {
        if self.halt {
            return;
        }

        if self.now.elapsed() < self.end_cycle {
            return;
        }

        let opcode = self.memory[self.pc as usize];
        let maddr = {
            let h = self.registers[4] as u16;
            let l = self.registers[5] as u16;

            ((h << 8) | l) as usize
        };

        let reg8f = |opcode: u8| {
            match opcode {
                // B
                0x08 => self.registers[0],
                // C
                0x09 => self.registers[1],
                // D
                0x0a => self.registers[2],
                // E
                0x0b => self.registers[3],
                // H
                0x0c => self.registers[4],
                // L
                0x0d => self.registers[5],
                // M
                0x0e => self.memory[maddr],
                // A
                0x0f => self.registers[6],

                _ => unreachable!(),
            }
        };
        let reg07 = |opcode: u8| {
            match opcode {
                // B
                0x00 => self.registers[0],
                // C
                0x01 => self.registers[1],
                // D
                0x02 => self.registers[2],
                // E
                0x03 => self.registers[3],
                // H
                0x04 => self.registers[4],
                // L
                0x05 => self.registers[5],
                // M
                0x06 => self.memory[maddr],
                // A
                0x07 => self.registers[6],

                _ => unreachable!(),
            }
        };

        let duration = match opcode {
            // NOP
            0x00 | 0x10 | 0x20 | 0x30 | 0x08 | 0x18 | 0x28 | 0x38 => {
                self.pc += 1;
                4.0
            }

            // CALL a
            0xcd | 0xdd | 0xed | 0xfd => self.call(true),
            // CZ
            0xcc => self.call(self.flag & 64 != 0),
            // CNZ
            0xc4 => self.call(self.flag & 64 == 0),
            // CC
            0xdc => self.call(self.flag & 1 != 0),
            // CNC
            0xd4 => self.call(self.flag & 1 == 0),
            // CPE
            0xec => self.call(self.flag & 4 != 0),
            // CP0
            0xe4 => self.call(self.flag & 4 == 0),
            // CM
            0xfc => self.call(self.flag & 128 != 0),
            // CP
            0xf4 => self.call(self.flag & 128 == 0),

            // INR B
            0x04 => self.incr(0),
            // INR C
            0x0c => self.incr(1),
            // INR D
            0x14 => self.incr(2),
            // INR E
            0x1c => self.incr(3),
            // INR H
            0x24 => self.incr(4),
            // INR L
            0x2c => self.incr(5),
            // INR A
            0x3c => self.incr(6),
            // INR M
            0x34 => {
                let m = self.memory[maddr];
                let res = m.wrapping_add(1);
                let ac = ((m & 0x0F) + (1 & 0x0F)) & 0x10 != 0;

                self.memory[maddr] = res;

                let z = res == 0;
                let p = (res.count_ones() % 2) == 0;

                // Affected flags get reset to 0
                self.flag &= 0b00000011;

                self.flag |= res & 0x80;
                self.flag |= u8::from(z) << 6;
                self.flag |= u8::from(ac) << 4;
                self.flag |= u8::from(p) << 2;

                self.pc += 1;
                10.0
            }

            // DCR B
            0x05 => self.dcr(0),
            // DCR C
            0x0d => self.dcr(1),
            // DCR D
            0x15 => self.dcr(2),
            // DCR E
            0x1d => self.dcr(3),
            // DCR H
            0x25 => self.dcr(4),
            // DCR L
            0x2d => self.dcr(5),
            // DCR A
            0x3d => self.dcr(6),
            // DCR M
            0x35 => {
                let m = self.memory[maddr];
                let res = m.wrapping_sub(1);
                let ac = calc_ac(m, 1);
                let p = (res.count_ones() % 2) == 0;

                self.memory[maddr] = res;

                self.flag &= 0b00000011;

                self.flag |= res & 0x80;
                self.flag |= u8::from(res == 0) << 6;
                self.flag |= u8::from(ac) << 4;
                self.flag |= u8::from(p) << 2;

                self.pc += 1;

                10.0
            }

            // JMP
            0xc3 | 0xcb => self.jump(true),
            // JZ
            0xca => self.jump(self.flag & 64 != 0),
            // JNZ
            0xc2 => self.jump(self.flag & 64 == 0),
            // JC
            0xda => self.jump(self.flag & 1 != 0),
            // JNC
            0xd2 => self.jump(self.flag & 1 == 0),
            // JPE
            0xea => self.jump(self.flag & 4 != 0),
            // JPO
            0xe2 => self.jump(self.flag & 4 == 0),
            // JM
            0xfa => self.jump(self.flag & 128 != 0),
            // JP
            0xf2 => self.jump(self.flag & 128 == 0),
            // PCHL
            0xe9 => {
                let h = self.registers[4] as u16;
                let l = self.registers[5] as u16;

                self.pc = (h << 8) | l;

                5.0
            }

            // RET
            0xc9 | 0xd9 => {
                self.ret(true);
                10.0
            }
            // RZ
            0xc8 => self.ret(self.flag & 64 != 0),
            // RNZ
            0xc0 => self.ret(self.flag & 64 == 0),
            // RC
            0xd8 => self.ret(self.flag & 1 != 0),
            // RNC
            0xd0 => self.ret(self.flag & 1 == 0),
            // RPE
            0xe8 => self.ret(self.flag & 4 != 0),
            // RPO
            0xe0 => self.ret(self.flag & 4 == 0),
            // RM
            0xf8 => self.ret(self.flag & 128 != 0),
            // RP
            0xf0 => self.ret(self.flag & 128 == 0),

            // LXI BC
            0x01 => self.lxi(0, 1),
            // LXI DE
            0x11 => self.lxi(2, 3),
            // LXI HL
            0x21 => self.lxi(4, 5),
            // LXI SP
            0x31 => {
                let hi = self.memory[(self.pc + 2) as usize] as u16;
                let low = self.memory[(self.pc + 1) as usize] as u16;

                self.sp = (hi << 8) | low;

                self.pc += 3;
                10.0
            }

            // MVI B
            0x06 => self.mvi(0),
            //MVI C
            0x0e => self.mvi(1),
            // MVI D
            0x16 => self.mvi(2),
            // MVI E
            0x1e => self.mvi(3),
            // MVI H
            0x26 => self.mvi(4),
            // MVI L
            0x2e => self.mvi(5),
            // MVI A
            0x3e => self.mvi(6),
            // MVI M
            0x36 => {
                self.memory[maddr] = self.memory[(self.pc + 1) as usize];

                self.pc += 2;

                10.0
            }

            // LDAX BC
            0x0a => {
                let b = self.registers[0] as u16;
                let c = self.registers[1] as u16;
                let addr = (b << 8) | c;

                self.registers[6] = self.memory[addr as usize];

                self.pc += 1;
                7.0
            }
            // LDAX DE
            0x1a => {
                let d = self.registers[2] as u16;
                let e = self.registers[3] as u16;
                let addr = (d << 8) | e;

                self.registers[6] = self.memory[addr as usize];

                self.pc += 1;
                7.0
            }

            // LDA
            0x3a => {
                let hi = self.memory[(self.pc + 2) as usize] as u16;
                let low = self.memory[(self.pc + 1) as usize] as u16;
                let addr = (hi << 8) | low;

                self.registers[6] = self.memory[addr as usize];

                self.pc += 3;
                13.0
            }

            // STA
            0x32 => {
                let hi = self.memory[(self.pc + 2) as usize] as u16;
                let low = self.memory[(self.pc + 1) as usize] as u16;
                let addr = (hi << 8) | low;

                self.memory[addr as usize] = self.registers[6];

                self.pc += 3;
                13.0
            }

            // STAX BC
            0x02 => {
                let b = self.registers[0] as u16;
                let c = self.registers[1] as u16;
                let addr = (b << 8) | c;

                self.memory[addr as usize] = self.registers[6];

                self.pc += 1;
                7.0
            }
            // STAX DE
            0x12 => {
                let d = self.registers[2] as u16;
                let e = self.registers[3] as u16;
                let addr = (d << 8) | e;

                self.memory[addr as usize] = self.registers[6];

                self.pc += 1;
                7.0
            }

            // LHLD
            0x2a => {
                let hi = self.memory[(self.pc + 2) as usize] as u16;
                let low = self.memory[(self.pc + 1) as usize] as u16;
                let addr = (hi << 8) | low;

                self.registers[5] = self.memory[addr as usize];
                self.registers[4] = self.memory[(addr + 1) as usize];

                self.pc += 3;
                16.0
            }

            // SHLD
            0x22 => {
                let hi = self.memory[(self.pc + 2) as usize] as u16;
                let low = self.memory[(self.pc + 1) as usize] as u16;
                let addr = (hi << 8) | low;

                self.memory[addr as usize] = self.registers[5];
                self.memory[(addr + 1) as usize] = self.registers[4];

                self.pc += 3;
                16.0
            }

            // XCHG
            0xeb => {
                //DH
                let temp = self.registers[2];
                self.registers[2] = self.registers[4];
                self.registers[4] = temp;

                // EL
                let temp = self.registers[3];
                self.registers[3] = self.registers[5];
                self.registers[5] = temp;

                self.pc += 1;
                5.0
            }

            // HLT
            0x76 => {
                self.pc += 1;
                self.halt = true;

                7.0
            }

            // INX BC
            0x03 => self.reg_cx(0, 1, |res| res.wrapping_add(1)),
            // INX DE
            0x13 => self.reg_cx(2, 3, |res| res.wrapping_add(1)),
            // INX HL
            0x23 => self.reg_cx(4, 5, |res| res.wrapping_add(1)),
            // INX SP
            0x33 => {
                self.sp = self.sp.wrapping_add(1);
                self.pc += 1;
                5.0
            }

            // DCX BC
            0x0b => self.reg_cx(0, 1, |res| res.wrapping_sub(1)),
            // DCX DE
            0x1b => self.reg_cx(2, 3, |res| res.wrapping_sub(1)),
            // DCX HL
            0x2b => self.reg_cx(4, 5, |res| res.wrapping_sub(1)),
            // DCX SP
            0x3b => {
                self.sp = self.sp.wrapping_sub(1);
                self.pc += 1;
                5.0
            }

            // RLC
            0x07 => {
                let acc = self.registers[6];

                self.flag = (self.flag & !1) | (acc >> 7);
                self.registers[6] = acc.rotate_left(1);

                self.pc += 1;
                4.0
            }

            // RRC
            0x0f => {
                let acc = self.registers[6];

                self.flag = (self.flag & !1) | (acc & 1);
                self.registers[6] = acc.rotate_right(1);

                self.pc += 1;
                4.0
            }

            // RAL
            0x17 => {
                let acc = self.registers[6];
                let carry = self.flag & 1;

                self.flag = (self.flag & !1) | (acc >> 7);
                self.registers[6] = (acc << 1) | carry;

                self.pc += 1;
                4.0
            }

            // RAR
            0x1f => {
                let acc = self.registers[6];
                let carry = self.flag << 7;

                self.flag = (self.flag & !1) | (acc & 1);
                self.registers[6] = (acc >> 1) | carry;

                self.pc += 1;
                4.0
            }

            // CMA
            0x2f => {
                self.registers[6] = !self.registers[6];
                self.pc += 1;
                4.0
            }

            // CMC
            0x3f => {
                self.flag ^= 1;

                self.pc += 1;
                4.0
            }

            // STC
            0x37 => {
                self.flag |= 1;
                self.pc += 1;
                4.0
            }

            // DAD BC
            0x09 => self.dad(0, 1),
            // DAD DE
            0x19 => self.dad(2, 3),
            // DAD HL
            0x29 => self.dad(4, 5),
            // DAD SP
            0x39 => {
                let sp = self.sp;

                let h = self.registers[4] as u16;
                let l = self.registers[5] as u16;
                let hl = (h << 8) | l;

                let carry = sp > 0xffff - hl;

                let res = hl.wrapping_add(sp);

                self.registers[4] = (res >> 8) as u8;
                self.registers[5] = (res & 0x00ff) as u8;
                self.flag &= !1;
                self.flag |= u8::from(carry);

                self.pc += 1;
                10.0
            }

            // DAA
            0x27 => {
                let mut cy = (self.flag & 1) != 0;
                let ac = (self.flag & 0b00010000) != 0;
                let mut correction = 0;

                let lsb = self.registers[6] & 0x0f;
                let msb = self.registers[6] >> 4;

                if ac || lsb > 9 {
                    correction += 0x06;
                }

                if cy || msb > 9 || (msb >= 9 && lsb > 9) {
                    correction += 0x60;
                    cy = true;
                }

                self.add(6, correction, false);

                self.flag &= !1;
                self.flag |= u8::from(cy);

                self.pc += 1;
                4.0
            }

            // POP BC
            0xc1 => self.pop(0, 1),
            // POP DE
            0xd1 => self.pop(2, 3),
            // POP HL
            0xe1 => self.pop(4, 5),
            // POP PSW
            0xf1 => {
                self.flag = (self.memory[(self.sp) as usize] & 0b11010101) | 0b0000_0010;
                self.registers[6] = self.memory[(self.sp + 1) as usize];
                self.sp += 2;

                self.pc += 1;
                10.0
            }

            // PUSH BC
            0xc5 => self.push(0, 1),
            // PUSH DE
            0xd5 => self.push(2, 3),
            // PUSH HL
            0xe5 => self.push(4, 5),
            // PUSH PSW
            0xf5 => {
                self.memory[(self.sp - 1) as usize] = self.registers[6];
                self.memory[(self.sp - 2) as usize] = self.flag;
                self.sp -= 2;

                self.pc += 1;
                11.0
            }

            // XTHL
            0xe3 => {
                let l = self.registers[5];
                self.registers[5] = self.memory[self.sp as usize];
                self.memory[self.sp as usize] = l;

                let h = self.registers[4];
                self.registers[4] = self.memory[(self.sp + 1) as usize];
                self.memory[(self.sp + 1) as usize] = h;

                self.pc += 1;
                18.0
            }

            // SPHL
            0xf9 => {
                let l = self.registers[5] as u16;
                let h = self.registers[4] as u16;
                let hl = (h << 8) | l;

                self.sp = hl;
                self.pc += 1;
                5.0
            }

            // IN
            0xdb => {
                let input = self.memory[(self.pc + 1) as usize];
                self.registers[6] = self.read(input);

                self.pc += 2;
                10.0
            }
            // OUT
            0xd3 => {
                let output = self.memory[(self.pc + 1) as usize];

                self.write(output, self.registers[6]);

                self.pc += 2;
                10.0
            }

            // EI
            0xfb => {
                self.interrupt = true;
                self.pc += 1;
                4.0
            }
            // DI
            0xf3 => {
                self.interrupt = false;
                self.pc += 1;
                4.0
            }

            // ADI
            0xc6 => {
                let imm = self.memory[(self.pc + 1) as usize];
                self.add(6, imm, false);

                self.pc += 2;
                7.0
            }

            // SUI
            0xd6 => {
                let imm = self.memory[(self.pc + 1) as usize];

                self.sub(6, imm, false);

                self.pc += 2;
                7.0
            }

            // ANI
            0xe6 => {
                let imm = self.memory[(self.pc + 1) as usize];
                self.ana(imm);

                self.pc += 2;
                7.0
            }

            // ORI
            0xf6 => {
                let imm = self.memory[(self.pc + 1) as usize];

                self.ora(imm);

                self.pc += 2;
                7.0
            }

            // ACI
            0xce => {
                let imm = self.memory[(self.pc + 1) as usize];
                let carry = (self.flag & 0x01) != 0;

                self.add(6, imm, carry);

                self.pc += 2;
                7.0
            }

            // SBI
            0xde => {
                let imm = self.memory[(self.pc + 1) as usize];
                let carry = self.flag & 0x01 != 0;
                self.sub(6, imm, carry);

                self.pc += 2;
                7.0
            }

            // XRI
            0xee => {
                let imm = self.memory[(self.pc + 1) as usize];
                self.xra(imm);

                self.pc += 2;
                7.0
            }

            // CPI
            0xfe => {
                let imm = self.memory[(self.pc + 1) as usize];
                self.cmp(imm);

                self.pc += 2;
                7.0
            }

            // RST 0
            0xc7 => {
                self.push_pc(1, 0);

                11.0
            }
            // RST 1
            0xcf => {
                self.push_pc(1, 1 * 8);

                11.0
            }
            // RST 2
            0xd7 => {
                self.push_pc(1, 2 * 8);

                11.0
            }
            // RST 3
            0xdf => {
                self.push_pc(1, 3 * 8);

                11.0
            }
            // RST 4
            0xe7 => {
                self.push_pc(1, 4 * 8);

                11.0
            }
            // RST 5
            0xef => {
                self.push_pc(1, 5 * 8);

                11.0
            }
            // RST 6
            0xf7 => {
                self.push_pc(1, 6 * 8);

                11.0
            }
            // RST 7
            0xff => {
                self.push_pc(1, 7 * 8);

                11.0
            }

            // MOV B,X
            0x40..=0x47 => {
                let cmp = opcode & 0x0f;

                let src = reg07(cmp);

                self.registers[0] = src;
                self.pc += 1;

                if cmp == 0x06 { 7.0 } else { 5.0 }
            }
            // MOV C,X
            0x48..=0x4f => {
                let cmp = opcode & 0x0f;

                let src = reg8f(cmp);

                self.registers[1] = src;
                self.pc += 1;

                if cmp == 0x0e { 7.0 } else { 5.0 }
            }
            // MOV D,X
            0x50..=0x57 => {
                let cmp = opcode & 0x0f;

                let src = reg07(cmp);

                self.registers[2] = src;
                self.pc += 1;

                if cmp == 0x06 { 7.0 } else { 5.0 }
            }
            // MOV E,X
            0x58..=0x5f => {
                let cmp = opcode & 0x0f;

                let src = reg8f(cmp);

                self.registers[3] = src;
                self.pc += 1;

                if cmp == 0x0e { 7.0 } else { 5.0 }
            }
            // MOV H,X
            0x60..=0x67 => {
                let cmp = opcode & 0x0f;

                let src = reg07(cmp);

                self.registers[4] = src;
                self.pc += 1;

                if cmp == 0x06 { 7.0 } else { 5.0 }
            }
            // MOV L,X
            0x68..=0x6f => {
                let cmp = opcode & 0x0f;

                let src = reg8f(cmp);

                self.registers[5] = src;
                self.pc += 1;

                if cmp == 0x0e { 7.0 } else { 5.0 }
            }
            // MOV M,X
            0x70..=0x77 => {
                let cmp = opcode & 0x0f;

                let src = reg07(cmp);

                self.memory[maddr] = src;
                self.pc += 1;

                7.0
            }
            // MOV A,X
            0x78..=0x7f => {
                let cmp = opcode & 0x0f;

                let src = reg8f(cmp);

                self.registers[6] = src;
                self.pc += 1;

                if cmp == 0x0e { 7.0 } else { 5.0 }
            }

            // ADD X
            0x80..=0x87 => {
                let cmp = opcode & 0x0f;

                let reg = reg07(cmp);
                self.add(6, reg, false);

                self.pc += 1;
                if cmp == 0x06 { 7.0 } else { 4.0 }
            }

            // ADC X
            0x88..=0x8f => {
                let cmp = opcode & 0x0f;

                let carry = (self.flag & 0x01) != 0;
                let reg = reg8f(cmp);
                self.add(6, reg, carry);

                self.pc += 1;
                if cmp == 0x0e { 7.0 } else { 4.0 }
            }

            // SUB X
            0x90..=0x97 => {
                let cmp = opcode & 0x0f;

                let reg = reg07(cmp);
                self.sub(6, reg, false);

                self.pc += 1;
                if cmp == 0x06 { 7.0 } else { 4.0 }
            }

            // SBB X
            0x98..=0x9f => {
                let cmp = opcode & 0x0f;
                let carry = (self.flag & 0x01) != 0;

                let reg = reg8f(cmp);
                self.sub(6, reg, carry);

                self.pc += 1;
                if cmp == 0x0e { 7.0 } else { 4.0 }
            }

            // ANA X
            0xa0..=0xa7 => {
                let cmp = opcode & 0x0f;

                let reg = reg07(cmp);
                self.ana(reg);

                self.pc += 1;
                if cmp == 0x06 { 7.0 } else { 4.0 }
            }

            // XRA X
            0xa8..=0xaf => {
                let cmp = opcode & 0x0f;

                let reg = reg8f(cmp);
                self.xra(reg);

                self.pc += 1;
                if cmp == 0x0e { 7.0 } else { 4.0 }
            }

            // ORA X
            0xb0..=0xb7 => {
                let cmp = opcode & 0x0f;

                let reg = reg07(cmp);
                self.ora(reg);

                self.pc += 1;
                if cmp == 0x06 { 7.0 } else { 4.0 }
            }

            // CMP X
            0xb8..=0xbf => {
                let cmp = opcode & 0x0f;

                let reg = reg8f(cmp);

                self.cmp(reg);

                self.pc += 1;
                if cmp == 0x0e { 7.0 } else { 4.0 }
            }
        } * PERIOD;

        self.end_cycle = Duration::from_secs_f32(duration);
        self.now = Instant::now();
    }

    /// Adds a value plus an optional carry flag to a register.
    fn add(&mut self, reg: usize, val: u8, cy: bool) {
        let reg_value = self.registers[reg];
        let result = reg_value.wrapping_add(val).wrapping_add(cy as u8);
        let p = (result.count_ones() % 2) == 0;

        self.flag = 0b00000010;
        self.flag |= result & 0x80;
        self.flag |= u8::from(result == 0) << 6;
        self.flag |= u8::from(carry(4, reg_value as u16, val as u16, cy)) << 4;
        self.flag |= u8::from(p) << 2;
        self.flag |= u8::from(carry(8, reg_value as u16, val as u16, cy));

        self.registers[reg] = result;
    }

    fn sub(&mut self, reg: usize, val: u8, cy: bool) {
        self.add(reg, !val, !cy);
        self.flag ^= 1;
    }

    /// XOR with register A
    fn xra(&mut self, val: u8) {
        self.registers[6] ^= val;
        let acc = self.registers[6];
        let p = (acc.count_ones() % 2) == 0;

        self.flag = 0b00000010;
        self.flag |= acc & 0x80;
        self.flag |= u8::from(acc == 0) << 6;
        self.flag |= u8::from(p) << 2;
    }

    /// OR with register A
    fn ora(&mut self, val: u8) {
        self.registers[6] |= val;

        let acc = self.registers[6];
        let p = (acc.count_ones() % 2) == 0;

        self.flag = 0b00000010;
        self.flag |= acc & 0x80;
        self.flag |= u8::from(acc == 0) << 6;
        self.flag |= u8::from(p) << 2;
    }

    /// Logical and with register A.
    fn ana(&mut self, val: u8) {
        let reg_value = self.registers[6];

        let result = reg_value & val;
        self.registers[6] = result;

        let p = (result.count_ones() % 2) == 0;

        self.flag = 0b00000010;
        self.flag |= result & 0x80;
        self.flag |= u8::from(result == 0) << 6;
        self.flag |= u8::from((reg_value | val) & 0x08 != 0) << 4;
        self.flag |= u8::from(p) << 2;
    }

    fn cmp(&mut self, val: u8) {
        let val = val as u16;
        let acc = self.registers[6] as u16;
        self.flag = 0b00000010;

        let result = acc.wrapping_sub(val);
        let ac = !(acc ^ result ^ val) & 0x10;
        self.flag |= u8::from((result >> 8) != 0);

        let result = (result & 0xff) as u8;
        let p = (result.count_ones() % 2) == 0;

        self.flag |= result & 0x80;
        self.flag |= u8::from(result == 0) << 6;
        self.flag |= u8::from(ac != 0) << 4;
        self.flag |= u8::from(p) << 2;
    }

    fn ret(&mut self, condition: bool) -> f32 {
        if condition {
            self.pop_pc();

            11.0
        } else {
            self.pc += 1;
            5.0
        }
    }

    fn pop_pc(&mut self) {
        let low = self.memory[(self.sp) as usize] as u16;
        let hi = self.memory[(self.sp + 1) as usize] as u16;

        self.pc = (hi << 8) | low;
        self.sp += 2;
    }

    fn push_pc(&mut self, next_pc: u16, new_pc: u16) {
        let ret = self.pc + next_pc;

        self.memory[(self.sp - 1) as usize] = (ret >> 8) as u8;
        self.memory[(self.sp - 2) as usize] = (ret & 0x00ff) as u8;

        self.sp -= 2;
        self.pc = new_pc;
    }

    fn call(&mut self, condition: bool) -> f32 {
        if condition {
            let hi = self.memory[(self.pc + 2) as usize] as u16;
            let low = self.memory[(self.pc + 1) as usize] as u16;
            let addr = (hi << 8) | low;

            self.push_pc(3, addr);

            17.0
        } else {
            self.pc += 3;

            11.0
        }
    }

    fn jump(&mut self, condition: bool) -> f32 {
        if condition {
            let hi = self.memory[(self.pc + 2) as usize] as u16;
            let low = self.memory[(self.pc + 1) as usize] as u16;
            let addr = (hi << 8) | low;

            self.pc = addr;
        } else {
            self.pc += 3;
        }

        10.0
    }

    fn incr(&mut self, register: usize) -> f32 {
        let reg = self.registers[register];
        let res = reg.wrapping_add(1);
        let ac = ((reg & 0x0F) + (1 & 0x0F)) & 0x10 != 0;

        self.registers[register] = res;

        let z = res == 0;
        let p = (res.count_ones() % 2) == 0;

        // Affected flags get reset to 0
        self.flag &= 0b00000011;

        self.flag |= res & 0x80;
        self.flag |= u8::from(z) << 6;
        self.flag |= u8::from(ac) << 4;
        self.flag |= u8::from(p) << 2;

        self.pc += 1;
        5.0
    }

    fn dcr(&mut self, register: usize) -> f32 {
        let reg = self.registers[register];
        let res = reg.wrapping_sub(1);
        let ac = calc_ac(reg, 1);
        let p = (res.count_ones() % 2) == 0;

        self.registers[register] = res;

        self.flag &= 0b00000011;

        self.flag |= res & 0x80;
        self.flag |= u8::from(res == 0) << 6;
        self.flag |= u8::from(ac) << 4;
        self.flag |= u8::from(p) << 2;

        self.pc += 1;
        5.0
    }

    fn lxi(&mut self, h: usize, l: usize) -> f32 {
        let low = self.memory[(self.pc + 1) as usize];
        let hi = self.memory[(self.pc + 2) as usize];

        self.registers[h] = hi;
        self.registers[l] = low;

        self.pc += 3;
        10.0
    }

    fn mvi(&mut self, register: usize) -> f32 {
        self.registers[register] = self.memory[(self.pc + 1) as usize];
        self.pc += 2;

        7.0
    }

    fn reg_cx(&mut self, h: usize, l: usize, op: fn(u16) -> u16) -> f32 {
        let hi = self.registers[h] as u16;
        let low = self.registers[l] as u16;
        let r = (hi << 8) | low;

        let res = op(r);

        self.registers[h] = (res >> 8) as u8;
        self.registers[l] = (res & 0x00ff) as u8;

        self.pc += 1;
        5.0
    }

    fn dad(&mut self, high: usize, low: usize) -> f32 {
        let high = self.registers[high] as u16;
        let low = self.registers[low] as u16;
        let one = (high << 8) | low;

        let h = self.registers[4] as u16;
        let l = self.registers[5] as u16;
        let hl = (h << 8) | l;

        let carry = one > 0xffff - hl;

        let res = hl.wrapping_add(one);

        self.registers[4] = (res >> 8) as u8;
        self.registers[5] = (res & 0x00ff) as u8;
        self.flag &= !1;
        self.flag |= u8::from(carry);

        self.pc += 1;
        10.0
    }

    fn push(&mut self, h: usize, l: usize) -> f32 {
        let h = self.registers[h];
        let l = self.registers[l];

        self.memory[(self.sp - 1) as usize] = h;
        self.memory[(self.sp - 2) as usize] = l;
        self.sp -= 2;

        self.pc += 1;
        11.0
    }

    fn pop(&mut self, h: usize, l: usize) -> f32 {
        self.registers[l] = self.memory[(self.sp) as usize];
        self.registers[h] = self.memory[(self.sp + 1) as usize];
        self.sp += 2;

        self.pc += 1;
        10.0
    }

    fn read(&mut self, port: u8) -> u8 {
        let Mode::Normal { reader, .. } = &mut self.mode else {
            return 0x00;
        };

        match reader.as_mut() {
            Some(read) => read(port),
            None => 0x00,
        }
    }

    pub fn set_reader(&mut self, reader: impl FnMut(u8) -> u8 + 'a) {
        match &mut self.mode {
            Mode::Normal { reader: prev, .. } => *prev = Some(Box::new(reader)),
            Mode::Testing(_) => {}
        }
    }

    fn write(&mut self, port: u8, byte: u8) {
        match &mut self.mode {
            Mode::Normal { writer, .. } => {
                if let Some(writer) = writer.as_mut() {
                    writer(port, byte);
                }
            }
            Mode::Testing(_) => {
                if port == 0 {
                    self.mode = Mode::Testing(false);
                    return;
                } else if port == 1 {
                    let operation = self.registers[1];

                    if operation == 2 {
                        let e = self.registers[3];
                        print!("{}", e as char);
                    } else {
                        let mut addr =
                            ((self.registers[2] as u16) << 8) | (self.registers[3] as u16);
                        print!("{}", self.memory[addr as usize] as char);

                        while self.memory[addr as usize] != 36 {
                            print!("{}", self.memory[addr as usize] as char);
                            addr += 1;
                        }
                    }
                }
            }
        }
    }

    pub fn set_write(&mut self, writer: impl FnMut(u8, u8) + 'a) {
        match &mut self.mode {
            Mode::Normal { writer: prev, .. } => *prev = Some(Box::new(writer)),
            Mode::Testing(_) => {}
        }
    }

    #[allow(dead_code)]
    fn testing(&self) -> bool {
        let Mode::Testing(testing) = &self.mode else {
            return false;
        };

        *testing
    }

    pub fn run(mut self) {
        loop {
            self.cycle()
        }
    }

    pub fn debug(&self) {
        println!(
            "\nPC: {}, SP: {}, Halt: {}, Interrupt: {}",
            self.pc, self.sp, self.halt, self.interrupt
        );

        println!(
            "Registers: B: 0x{:02x}, C: 0x{:02x}, D: 0x{:02x}, E: 0x{:02x}, H: 0x{:02x}, L: 0x{:02x}, A: 0x{:02x}",
            self.registers[0],
            self.registers[1],
            self.registers[2],
            self.registers[3],
            self.registers[4],
            self.registers[5],
            self.registers[6],
        );

        println!(
            "Flags: S: {}, Z: {}, A: {}, P: {}, C: {}",
            self.flag >> 7,
            (self.flag >> 6) & 1,
            (self.flag >> 4) & 1,
            (self.flag >> 2) & 1,
            self.flag & 1,
        );
    }
}

/// returns if there was a carry between bit "bit_no" and "bit_no - 1" when
/// executing "a + b + cy"
fn carry(bit_no: u8, a: u16, b: u16, carry: bool) -> bool {
    let result = a.wrapping_add(b).wrapping_add(carry as u16);
    let carry = result ^ a ^ b;

    return (carry & (1 << bit_no)) != 0;
}

/// Calculates the Auxillary Carry from `x-y`
fn calc_ac(x: u8, y: u8) -> bool {
    (((!y).wrapping_add(1) & 0x0f) + (x & 0x0f)) & 0x10 != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read;

    fn prep(program: &[u8]) -> Emulator<'_> {
        let start = 0x100;
        let mut memory = [0; MEM_SIZE];
        let len = program.len().min(MEM_SIZE - start);

        memory[start..start + len].copy_from_slice(&program[..len]);

        memory[0x0000] = 0xd3;
        memory[0x0001] = 0x00;

        memory[0x0005] = 0xd3;
        memory[0x0006] = 0x01;
        memory[0x0007] = 0xc9;

        Emulator::new_test(memory, start as u16)
    }

    fn test(test: &str) {
        let rom = read(format!("./tests/{test}.COM")).unwrap();
        let mut emulator = prep(&rom);

        println!("\n**** Testing {test}.COM");

        let mut ins = 0usize;

        while emulator.testing() {
            ins += 1;
            emulator.cycle();
        }

        println!("\n**** {ins} instructions");
    }

    #[test]
    fn tests() {
        //test("8080PRE");
        //test("TST8080");
        test("CPUTEST");
        //test("8080EXM");
    }
}
