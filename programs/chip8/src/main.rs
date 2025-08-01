#![allow(unused_imports, dead_code)]
use intel8080::{Bus, CPU, MEM_SIZE, RATE};
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use pixels::{Pixels, SurfaceTexture};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::{File, read};
use std::io::{self, BufWriter, Read, Write};
use std::io::{BufReader, Cursor};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const START: u16 = 0x18e2;
const WIDTH: usize = 64;
const HEIGHT: usize = 32;
const SCALE: usize = 10;
const I: usize = 0x10;
const GFX: usize = 4116;

#[rustfmt::skip]
const CHIP_FONTSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

#[rustfmt::skip]
const EMULATOR: [u8; 1416] = [
         // XREG: 6196
        0x78,
        0xe6,
        0x0f,
        0xc9,

        // YREG: 6200, 6203
        // Use YREG2
        0xc3, // JMP
        0x2c,
        0x1a,
        0x00,

        //Unknown: 6204, 6211
        0x78, 
        0xd3,
        0x01,
        0x79,
        0xd3,
        0x01,
        0x00,
        0x76,

        // OPP PREP: 6212, 6227
        0xcd,
        0x2c,
        0x1a,
        0xe5, // PUSH HL,
        0x6f, 
        0x26,
        0x00,
        0x5e, // MOV E, M
        0xcd,
        0x34,
        0x18,
        0x57, // MOV D, A
        0x6f, 
        0x7e,
        0xe1, // POP HL
        0xc9,

        // INCR PC: 6228, 6235
        // 0x7b,
        // 0x23, 
        // 0xd6,
        // 0x01,
        // 0xc8,
        // 0xc3,
        // 0x55,
        // 0x18,
        0x7b,
        0xd6,
        0x01,
        0xd8,
        0x23,
        0xc3, // JMP
        0x55,
        0x18,

        // KEY: 6236, 6263
        0xcd,
        0x34,
        0x18,
        0xe5,
        0x6f,
        0x26,
        0x00,
        0x7e,
        0xe1,
        0xd3,
        0x02,
        0xdb,
        0x02,
        0xba,
        0x1e,
        0x01,
        0xc2,
        0x71,
        0x18,
        0x1e,
        0x03,
        0xcd,
        0x54,
        0x18,
        0xc9,
        0x00,
        0x00,
        0x00,

        // REG: 6264 
        0xe5, 
        0x26,
        0x00,
        0x6b, // MOV L, E
        0x7e, // MOV A, M
        0xe1,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // IDENTIFY: 6276
        0x16,
        0x00,
        0x78,
        0xe6, // ANI
        0xf0,
        0x0f,
        0x0f, // RRC
        0x0f,
        0x0f,
        0xba, // CMP D
        0xca, // JZ-0
        0x14,
        0x19, 
        0x14, // INR D 
        0xba,
        0xca, // JZ-1
        0x50,
        0x19,
        0x14, 
        0xba,
        0xca, // JZ-2
        0x60, 
        0x19, 
        0x14, 
        0xba,
        0xca, // JZ-3
        0x88, 
        0x19, 
        0x14, 
        0xba,
        0xca, // JZ-4
        0x9c, 
        0x19, 
        0x14, 
        0xba,
        0xca, // JZ-5
        0xb0, 
        0x19, 
        0x14, 
        0xba,
        0xca, // JZ-6
        0xcc, 
        0x19, 
        0x14, 
        0xba,
        0xca, // JZ-7
        0xdc, 
        0x19, 
        0x14, 
        0xba,
        0xca, // JZ-8
        0xf4, 
        0x19, 
        0x14, 
        0xba,
        0xca, // JZ-9
        0x30, 
        0x1b, 
        0x14, 
        0xba,
        0xca, // JZ-A
        0x4c, 
        0x1b, 
        0x14, 
        0xba,
        0xca, // JZ-B
        0x6c, 
        0x1b, 
        0x14, 
        0xba,
        0xca, // JZ-C
        0x88, 
        0x1b, 
        0x14, 
        0xba,
        0xca, // JZ-D
        0x9c, 
        0x1b, 
        0x14, 
        0xba,
        0xca, // JZ-E
        0xec, 
        0x1b, 
        0x14, 
        0xba,
        0xca, // JZ-F
        0x0c, 
        0x1c, 
        0xcd, // CALL unknown
        0x3c,
        0x18,
        0xc9,


        // CYCLE: 6368
        0x02, // Start address for chip pc
        0x00,
        0x26,
        0x18,
        0x2e,
        0xe0,
        0x56, // MOV D, M
        0x2c, // INR L
        0x7e, // MOV A, M
        0xc6, // ADI 
        0x14,
        0xd2, // JNC
        0xef,
        0x18,
        0x14, // INR D
        0x5f, // MOV E, A
        0xeb, // XCHG

        0x46, // MOV B, M
        0x23, // INX HL
        0x4e, // MOV C, M
        0xcd,
        0x84,
        0x18,

        0x79, // MOV A, C
        0x1e,
        0x00,
        0xfe,
        0xe0,
        0xc2, // JNZ
        0x04,
        0x19,
        0x1e,
        0x01,
        0xc3, // JMP
        0x0e,
        0x19,
        0x78, // MOV A, B
        0xe6, // ANI
        0xf0,
        0xfe,
        0xd0,
        0xc2, // JNZ
        0x0e,
        0x19,
        0x1e,
        0x01,
        0x7b, // MOV A, E
        0xd3,
        0x08,
        0xc3, // JMP
        0xf1,
        0x18,

        // JZ-0: 6420
        0x79, // MOV A, C
        0xfe, // CPI
        0xe0,
        0xc2, // JNZ
        0x3f, //(EE)
        0x19,
        0xe5, // 00E0: PUSH HL
        0x26, // MVI H
        0x10,
        0x2e, // MVI L
        0x14,
        0x16, // MVI D
        0x08,
        0x1e, // MVI E
        0x00,
        0x3e, // MVI A
        0x00,
        0xbb, // CMP E
        0xc2, // JNZ
        0x38, // (Jump to inx hl, dcx de)
        0x19,
        0xba, // CMP D
        0xc2, // JNZ
        0x38, // (Jump to inx hl, dcx de)
        0x19,
        0xe1, // POP HL
        0x3e,
        0x01,
        0xd3, // OUT DRAW
        0x08,
        0x23,
        0xc9, // RET
        0x00,
        0x00,
        0x00,
        0x00,
        0x36, //MVI M
        0x00,
        0x23, // INX H
        0x1b,
        0xc3, // JMP
        0x25,
        0x19,
        0x26, // 00EE: MVI H
        0x00,
        0x2e,
        0x12,
        0x35, // DCR M
        0x35,
        0x7e, // MOV A, M(SP)
        0xc6, // ADI
        0x14,
        0x26,
        0x18,
        0x6f, // MOV L,A
        0x56,
        0x23, // INX HL
        0x5e,
        0xeb, // XCHG
        0xc9,

        // JZ-1 1NNN: 6480,
        0x78, // MOV A, B
        0xe6, // ANI 
        0x0f,
        0x67,
        0x79, // MOV A, C
        0xc6,
        0x14,
        0xd2, // JNC
        0x5b,
        0x19,
        0x24, // INR H
        0x6f,
        0xc9,
        0x00,
        0x00,
        0x00,

        // JZ-2 2NNN: 6496
        0x23,
        0xeb, //XCHG
        0x26, 
        0x00,
        0x2e, // MVI L
        0x12,
        0x7e, // MOV A, M(SP)
        0xc6,
        0x14,
        0x26, // MVI H
        0x18,
        0x6f,
        0x72, // MOV M, D
        0x23,
        0x73,
        0x26, // MVI H
        0x00,
        0x2e,
        0x12,
        0x34, // INR M
        0x34, 
        0xcd, // CALL JZ-1
        0x50,
        0x19,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-3 3xkk: 6536
        0xcd,
        0x34,
        0x18,
        0x5f, // MOV E, A
        0xcd,
        0x78,
        0x18,
        0xb9, // CMP C
        0x1e,
        0x01,
        0xc2, // JNZ
        0x97,
        0x19,
        0x1e, // MVI 
        0x03,
        0xcd,
        0x54,
        0x18,
        0xc9,
        0x00,

        // JZ-4 4xkk: 6556
        0xcd, // CALL XREG
        0x34,
        0x18,
        0x5f,
        0xcd, // CALL REG
        0x78,
        0x18,
        0xb9,
        0x1e, // MVI E
        0x01,
        0xca, // JZ
        0xab,
        0x19,
        0x1e,
        0x03,
        0xcd,
        0x54,
        0x18,
        0xc9,
        0x00,

        // JZ-5 5xy0: 6576
        0xcd,
        0x34,
        0x18,
        0x5f, // MOV E, A 
        0xcd,
        0x78,
        0x18,
        0x57, // MOV D, A
        0xcd,
        0x2c,
        0x1a,
        0x5f,
        0xcd, // REG
        0x78,
        0x18,
        0xba, // CMP D
        0x1e,
        0x01,
        0xc2, // JNZ
        0xc7,
        0x19,
        0x1e,
        0x03,
        0xcd, // INCR PC
        0x54,
        0x18,
        0xc9,
        0x00,


        // JZ-6 6xkk: 6604
        0xcd,
        0x34,
        0x18,
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x6f, // MOV L, A
        0x71, // MOV M, C
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-7 7xkk: 6620
        0xcd,
        0x34,
        0x18,
        0x5f, // MOV E, A
        0xcd,
        0x78,
        0x18,
        0x81, // ADD C
        0x57,
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x6b, // MOV L, E
        0x72, // MOV M, D
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-8: 6644
        0x16,
        0x00,
        0x79, // MOV A, C
        0xe6,
        0x0f,
        0xba,
        0xca, // JZ-8xy0
        0x34,
        0x1a,
        0x14, // INR D 
        0xba,
        0xca, // JZ-8xy1
        0x4c,
        0x1a,
        0x14,
        0xba,
        0xca, // JZ-8xy2
        0x60,
        0x1a,
        0x14,
        0xba,
        0xca, // JZ-8xy3
        0x74,
        0x1a,
        0x14,
        0xba,
        0xca, // JZ-8xy4
        0x88,
        0x1a,
        0x14,
        0xba,
        0xca, // JZ-8xy5
        0xa4,
        0x1a,
        0x14,
        0xba,
        0xca, // JZ-8xy6
        0xc0,
        0x1a,
        0x14,
        0xba,
        0xca, // JZ-8xy7
        0xe4,
        0x1a,
        0x16,
        0x0e,
        0xba,
        0xca, // JZ-8xye
        0x08,
        0x1b,
        0xcd, // UNKNOWN
        0x3c,
        0x18,
        0xc9,
        0x00,
        0x00,

        // YREG2: 6700
        0x79,
        0xe6,
        0xf0,
        0x0f,
        0x0f,
        0x0f,
        0x0f,
        0xc9,

        // JZ-8xy0: 6708
        0xcd, // YREG
        0x2c,
        0x1a,
        0x5f, // MOV E, A
        0xcd,
        0x78,
        0x18,
        0x57, // MOV D, A
        0xcd,
        0x34,
        0x18,
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x6f, 
        0x72, // MOV M, D
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-8xy1: 6732
        0xcd,
        0x44,
        0x18,
        0xb3, // ORA E
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x6a, // MOV L, D
        0x77,
        0x2e,
        0x0f,
        0x36, // MVI M
        0x00,
        0xe1, // POP HL
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-8xy2: 6752
        0xcd,
        0x44,
        0x18,
        0xa3, // ANA E
        0xe5,
        0x26,
        0x00,
        0x6a, 
        0x77, // MOV M, A
        0x2e,
        0x0f,
        0x36,
        0x00,
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-8xy3: 6772
        0xcd,
        0x44,
        0x18,
        0xab, // XRA E
        0xe5,
        0x26,
        0x00,
        0x6a, 
        0x77, // MOV M, A
        0x2e,
        0x0f,
        0x36,
        0x00,
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-8xy4: 6792
        0xcd,
        0x44,
        0x18,
        0x83, // ADD E
        0xe5,
        0x26,
        0x00,
        0x6a,
        0x77, // MOV M, A
        0x2e,
        0x0f,
        0x1e, // MVI E
        0x01,
        0xda, // JC
        0x9a,
        0x1a,
        0x1e,
        0x00,
        0x73, // MOV M, E
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-8xy5: 6820
        0xcd,
        0x44,
        0x18,
        0x93, // SUB E
        0xe5,
        0x26,
        0x00,
        0x6a, // MOV L, D
        0x77, 
        0x2e,
        0x0f,
        0x1e,
        0x00,
        0xda, // JC
        0xb6,
        0x1a,
        0x1e,
        0x01,
        0x73, // MOV M, E
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // R-SHIFT: 6848
        // 0x57,
        // 0x3e,
        // 0x00,
        // 0xbb, // CMP E
        // 0xc8, // RZ
        // 0xaf, // XRA
        // 0x7a,
        // 0x1f, // RAR
        // 0x1d,
        // 0xc3, // JMP
        // 0xc0, 
        // 0x1a,

        // L-SHIFT: 6860
        // 0x57,
        // 0x3e,
        // 0x00,
        // 0xbb, // CMP E
        // 0xc8, // RZ
        // 0xaf, // XRA
        // 0x7a,
        // 0x17, // RAL
        // 0x1d,
        // 0xc3, // JMP
        // 0xcc, 
        // 0x1a,


        // JZ-8xy6: 6848
        0xcd,
        0x2c,
        0x1a,
        0x5f, // MOV E, A
        0xcd,
        0x78,
        0x18,
        0x5f,
        0x37, // STC
        0x3f, // CMC
        0x1f, // RAR
        0x57,
        0xcd, // CALL XREG
        0x34,
        0x18,
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x6f, 
        0x72, // MOV M, D
        0x2e,
        0x0f,
        0x7b, // MOV A, E
        0xe6,
        0x01,
        0x77, // MOV M, A
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-8xy7: 6884
        0xcd,
        0x34,
        0x18,
        0x57, // MOV D, A
        0xe5,
        0x26,
        0x00,
        0x6f,
        0x5e, // MOV E, M
        0xcd,
        0x2c,
        0x1a,
        0x6f, // MOV L, A
        0x7e, // MOV A, M
        0x93,
        0x6a, // MOV L, D
        0x77,
        0x2e,
        0x0f,
        0x1e,
        0x00,
        0xda, //JC
        0xfe,
        0x1a,
        0x1e,
        0x01,
        0x73, // MOV M, E
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-8xye: 6920
        0xcd,
        0x2c,
        0x1a,
        0x5f,
        0xcd,
        0x78,
        0x18,
        0x5f, // MOV E, A
        0x37, // STC
        0x3f, // CMC
        0x17, // RAL
        0x57,
        0xcd,
        0x34,
        0x18,
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x6f, // MOV L, A
        0x72, 
        0x2e,
        0x0f,
        0x7b, // MOV A, E
        0x07, // RLC
        0x1e,
        0x00,
        0xd2, // JNC
        0x27,
        0x1b,
        0x1e,
        0x01,
        0x73,
        0xe1, // POP HL
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-9xy0: 6960
        0xcd,
        0x34,
        0x18,
        0x5f,
        0xcd,
        0x78,
        0x18,
        0x57, // MOV D,A
        0xcd,
        0x2c,
        0x1a,
        0x5f,
        0xcd,
        0x78,
        0x18,
        0xba,
        0x1e,
        0x01,
        0xca, // JZ
        0x47,
        0x1b,
        0x1e,
        0x03,
        0xcd,
        0x54,
        0x18,
        0xc9,
        0x00,

        // JZ-Annn: 6988
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x2e,
        0x11,
        0x79, // MOV A, C
        0xc6, // ADI
        0x14,
        0x1e,
        0x00,
        0xd2, // JNC
        0x5b,
        0x1b,
        0x1e,
        0x01,
        0x77, // MOV M, A
        0x2d, // DCR L
        0x78,
        0xe6,
        0x0f,
        0x83, // ADD E
        0x77,
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-Bnnn: 7020
        0x1e,
        0x00,
        0xcd,
        0x78,
        0x18,
        0x1e,
        0x00,
        0xc6, // ADI
        0x14,
        0xd2, // JNC
        0x79,
        0x1b,
        0x1c, // INR E    
        0x81, // ADD C
        0xd2, // JNC
        0x7e,
        0x1b,
        0x1c, // INR E    
        0x6f,
        0x78, // MOV A, B
        0xe6,
        0x0f,
        0x83, // ADD E
        0x67, // MOV H, A
        0xc9,
        0x00,
        0x00,
        0x00,


        // JZ-Cxkk: 7048
        0xcd,
        0x34,
        0x18,
        0x57, // MOV D, A
        0xdb, // IN RAND
        0x01,
        0xa1,
        0xe5,
        0x26,
        0x00,
        0x6a, // MOV L, D
        0x77,
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-Dxyn: 7068
        0xcd,
        0x34,
        0x18,
        0x5f,
        0xcd,
        0x78,
        0x18,
        0xd3, // OUT DX
        0x05,
        0xcd,
        0x2c,
        0x1a,
        0x5f,
        0xcd,
        0x78,
        0x18,
        0xd3,
        0x06, // OUT DY
        0x79,
        0xe6,
        0x0f,
        0xd3, // OUT DN
        0x07,
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x2e,
        0x0f,
        0x36, // MVI M
        0x00,

        0xdb, // IN END
        0x07,
        0xfe,
        0x01,
        0xca, // JZ
        0xe0, 
        0x1b,

        0xdb, // IN POS
        0x09,
        0x6f, // MOV L, A
        0xdb,
        0x0a,
        0x67, // MOV H, A
        0x7e,
        0xee,
        0x01,
        0x77,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        0xdb, // IN CLIP
        0x08,
        0xfe, // CPI 
        0x01,
        0xc2, // JNZ
        0xba,
        0x1b,
        0x26,
        0x00,
        0x2e,
        0x0f,
        0x37, // MVI M
        0x01,
        0xc3, // JMP
        0xba,
        0x1b,

        0x3e, // MVI A
        0x01,
        0xd3, // OUT DRAW 
        0x08,
        0xe1, // POP HL
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-E: 7148
        0x79,
        0xfe, // CPI
        0x9e,
        0xca, // JZ-EX9E
        0xfc,
        0x1b,
        0xfe,
        0xa1,
        0xca, // JZ-EXAI
        0x04,
        0x1c,
        0xcd,
        0x3c,
        0x18,
        0xc9,
        0x00,

        // JZ-EX9E: 7164
        0x16,
        0x01,
        0xcd,
        0x5c,
        0x18,
        0xc9,
        0x00,
        0x00,
            
        // JZ-EXA1: 7172
        0x16,
        0x00,
        0xcd,
        0x5c,
        0x18,
        0xc9,
        0x00,
        0x00,

        // JZ-F: 7180
        0x79,
        0xfe,
        0x07,
        0xca, // JZ-FX07
        0x40,
        0x1c,
        0xfe,
        0x0a,
        0xca, // JZ-FX0A
        0x54,
        0x1c,
        0xfe,
        0x15,
        0xca, // JZ-FX15
        0x90,
        0x1c,
        0xfe,
        0x18,
        0xca, // JZ-FX18
        0xa0,
        0x1c,
        0xfe,
        0x1e,
        0xca, // JZ-FX1E
        0xb0,
        0x1c,
        0xfe,
        0x29,
        0xca, // JZ-FX29
        0xcc,
        0x1c,
        0xfe,
        0x33,
        0xca, // JZ-FX33
        0x00,
        0x1d,
        0xfe,
        0x55,
        0xca, // JZ-FX55
        0x44,
        0x1d,
        0xfe,
        0x65,
        0xca, // JZ-FX65
        0x80,
        0x1d,
        0xcd,
        0x3c,
        0x18,
        0x00,
        0x00,
        0x00,

        // JZ-FX07: 7128
        0xcd,
        0x34,
        0x18,
        0x57, // MOV D, A
        0xdb, // IN DELAY
        0x03,
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x6a, // MOV L, D
        0x77,
        0xe1, // POP HL
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-FX0A: 7252
        0x3e,
        0x01,
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x2e,
        0x13,
        0xbe, // CMP M
        0xe1,
        0xc2, // JNZ
        0x7c,
        0x1c,
        0xcd,
        0x34,
        0x18,
        0x5f,
        0xcd, // CALL REG
        0x78,
        0x18,
        0xd3, // OUT KEY
        0x02,
        0xdb, // IN KEY
        0x02,
        0xfe,
        0x00,
        0xc0, // RNZ
        0xe5,
        0x26,
        0x00,
        0x2e,
        0x13,
        0x36,
        0x00,
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0xdb, // IN KEY WAIT
        0x05,
        0xfe,
        0xff,
        0xc8, // RZ
        0x57,
        0xcd,
        0x34,
        0x18,
        0xe5,
        0x26,
        0x00,
        0x6f, // MOV L, A
        0x72,
        0x2e,
        0x13,
        0x36,
        0x01,
        0xe1,
        0xc9,

        // JZ-FX15: 7312
        0xcd,
        0x34,
        0x18,
        0x5f,
        0xcd,
        0x78,
        0x18,
        0xd3, // OUT DELAY
        0x03,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-FX18: 7328
        0xcd,
        0x34,
        0x18,
        0x5f,
        0xcd,
        0x78,
        0x18,
        0xd3, // OUT SOUND
        0x04,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-FX1E: 7344
        0xcd,
        0x34,
        0x18,
        0x5f,
        0xcd,
        0x78,
        0x18,
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x2e,
        0x11,
        0x86, // ADD M
        0x77,
        0xd2, // JNC
        0xc3,
        0x1c,
        0x2d,
        0x34, // INR M
        0xe1, // POP HL
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-FX29: 7372
        0xcd,
        0x34,
        0x18,
        0x5f,
        0xcd,
        0x78,
        0x18,
        0x57, // MOV D, A
        0x1e,
        0x00,
        0x37, // STC
        0x3f, // CMC
        0x17, // RAL
        0xd2, // JNC
        0xdd,
        0x1c,
        0x1c, // INR E
        0x4f, // MOV C, A
        0x7b, // MOV A, E
        0x07, // RLC,
        0x5f, // MOV E, A
        0x79, // MOV A, C
        0x37, // STC
        0x3f, // CMC
        0x17, // RAL
        0xd2, // JNC
        0xe9,
        0x1c,
        0x1c,
        0x82, // ADD D
        0xd2, // JNC
        0xee,
        0x1c,
        0x1c,
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x2e,
        0x11,
        0x77, // MOV M, A
        0x2d, 
        0x73, // MOV M, E
        0xe1, // POP HL
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-FX33: 7424
        0xcd,
        0x34,
        0x18,
        0x5f,
        0xcd,
        0x78,
        0x18,
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x2e,
        0x11,
        0x5e, // MOV E, M
        0x2d,
        0x56, // MOV D, M
        0xeb, // XCHG
        0x57, // MOV D, A
        0x1e,
        0x00,
        0xfe,
        0x64,
        0xda, // JC
        0x1e,
        0x1d,
        0xd6, // SUI
        0x64,
        0x1c,
        0xc3, // JMP
        0x13,
        0x1d,
        0x73, // MOV M, E
        0x1e,
        0x00,
        0xfe,
        0x0a,
        0xda, // JC
        0x2c,
        0x1d,
        0xd6,
        0x0a,
        0x1c, // INR E
        0xc3, // JMP
        0x21,
        0x1d,
        0x2c, // INR L
        0x73, 
        0x1e,
        0x00,
        0xfe,
        0x01,
        0xda, // JC
        0x3b,
        0x1d,
        0xd6,
        0x01,
        0x1c,
        0xc3, // JMP
        0x30,
        0x1d,
        0x2c,
        0x73, // MOV M, E
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-FX55: 7492
        0xcd,
        0x34,
        0x18,
        0x47, // MOV B, A
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x2e,
        0x10,
        0x56, // MOV D, M
        0x2c,
        0x5e, // MOV E, M
        0x26,
        0x00,
        0x2e,
        0x00,
        0x3e, // MVI A
        0x00,
        0xb8, // CMP B
        0xd2, // JNC
        0x64,
        0x1d,
        0x4e, // MOV C, M
        0xeb, // XCHG
        0x71, // MOV M, C
        0xeb,
        0x3c, // INR A
        0x13, // INX DE
        0x23, // INX HL
        0xc3, // JMP
        0x56,
        0x1d,
        0x4e, // MOV C, M
        0xeb, // XCHG
        0x71, // MOV M, C
        0x26, // MVI H
        0x00,
        0x2e,
        0x11,
        0x1e,
        0x00,
        0x78, // MOV A, B
        0x3c,
        0x86, // ADD M
        0xd2, // JNC
        0x74,
        0x1d,
        0x1c, // INR E
        0x77, // MOV M, A
        0x2d, // DCR L
        0x7e, // MOV A, M
        0x83, // ADD E
        0x77,
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,

        // JZ-FX65: 7552
        0xcd,
        0x34,
        0x18,
        0x47, // MOV B, A
        0xe5, // PUSH HL
        0x26,
        0x00,
        0x2e,
        0x10,
        0x56, // MOV D, M
        0x2c,
        0x5e, // MOV E, M
        0x26,
        0x00,
        0x2e,
        0x00,
        0xeb, // XCHG
        0x3e, // MVI A
        0x00,
        0xb8, // CMP B
        0xd2, // JNC
        0xa1,
        0x1d,
        0x4e, // MOV C, M
        0xeb, // XCHG
        0x71, // MOV M, C
        0xeb,
        0x3c, // INR A
        0x13, // INX DE
        0x23, // INX HL
        0xc3, // JMP
        0x93,
        0x1d,
        0x4e, // MOV C, M
        0xeb, // XCHG
        0x71, // MOV M, C
        0x26, // MVI H
        0x00,
        0x2e,
        0x11,
        0x1e,
        0x00,
        0x3c,
        0x86, // ADD M
        0xd2, // JNC
        0xb0,
        0x1d,
        0x1c, // INR E
        0x77, // MOV M, A
        0x2d, // DCR L
        0x7e, // MOV A, M
        0x83, // ADD E
        0x77,
        0xe1,
        0x23,
        0xc9,
        0x00,
        0x00,
        0x00,
        0x00,

    ];

// fn main() -> Result<(), Error> {
fn test_main() -> Result<(), Error> {
    let mut args = std::env::args();
    let _ = args.next();
    let path = args.next().expect("Missing path to Chip8 ROM");
    let _chip8 = read(path)?;
    let chip8 = [0xf5, 0x55, 0xa2, 0x58, 0xf5, 0x65];

    let mut cpu = load_rom(&chip8);
    let mut chip = Chip::new();
    let mut count = 0;

    // let start = 6364;
    // let len = 52;
    // let end = start + len;
    //
    // for i in start..end {
    //     println!("{:02x}", cpu.memory()[i]);
    // }

    // println!("{:02x}", cpu.memory()[6370]);
    // println!("{:02x}", cpu.memory()[7540]);
    // println!("{:02x}", cpu.memory()[0x1d3e]);
    // println!("{:02x}", cpu.memory()[0x1d3f]);

    // println!("PC: 0x{:02x}{:02x}", cpu.register(4), cpu.register(5));
    // 12362
    // 23217
    // 22387 // SP still 0
    // for _ in 0..22350 {
    //     cpu.cycle(&mut chip);
    //     count += 1;
    // }
    for _ in 0..401 {
        cpu.cycle(&mut chip);
        count += 1;
    }

    loop {
        print!(">> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();

        if let Err(error) = io::stdin().read_line(&mut input) {
            eprintln!("{error}");
            continue;
        }

        let mut input = input.split_whitespace();
        let command = input.next();

        match command {
            Some("q") => {
                break;
            }
            Some("d") => {
                debug_chip(&cpu);
                println!("{:?}", &cpu.memory()[6164..6168]);
            }
            Some("f") => {
                let start = 0x14;
                println!("{:?}", &cpu.memory()[start..start + 5])
            }
            Some("g") => {
                println!("{:?}", &cpu.memory()[4116..4120]);
            }
            Some("m") => {
                println!("{:?}", &cpu.memory()[532..541]);
            }
            Some("s") => {
                println!("0x{:02x}", &cpu.memory()[18]);
            }
            _ => {
                count += 1;
                println!("Count: {count}");
                cpu.cycle(&mut chip);
                cpu.debug();
            }
        }

        println!("");
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    // fn main_main() -> Result<(), Error> {
    let mut args = std::env::args();
    let _ = args.next();
    let path = args.next().expect("Missing path to Chip8 ROM");
    let chip8 = read(path)?;
    let mut cpu = load_rom(&chip8);
    let mut chip = Chip::new();

    let mut window = Window::new(
        "CHIP-8 Emulator",
        WIDTH * SCALE,
        HEIGHT * SCALE,
        WindowOptions {
            resize: false,
            scale: Scale::X1,
            ..WindowOptions::default()
        },
    )?;

    // for _ in 0..23217 {
    //     cpu.cycle(&mut chip);
    // }
    // draw(&mut window, &cpu.memory()[4116..=6163])?;

    // let (_stream, stream_handle) = OutputStream::try_default()?;
    // let sink = Sink::try_new(&stream_handle)?;
    // sink.append(SineWave::new(440.0));

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let pressed = window
            .get_keys_pressed(KeyRepeat::No)
            .into_iter()
            .filter_map(map_key)
            .map(|key| (key, true));
        let released = window
            .get_keys_released()
            .into_iter()
            .filter_map(map_key)
            .map(|key| (key, false));

        for (key, pressed) in pressed.chain(released) {
            chip.set_key(key, pressed)
        }

        let mut cycles = 0;

        while cycles < 5 * (RATE / 10_000) {
            let spent = cpu.cycle(&mut chip) as u32;
            cycles += spent;
            chip.step();
        }

        if chip.draw {
            draw(&mut window, &cpu.memory()[4116..=6163])?;
        } else {
            window.update();
        }

        // window.update();

        // if self.sound {
        //     sink.play()
        // } else {
        //     sink.pause()
        // }
    }

    Ok(())
}

fn load_rom(chip8: &[u8]) -> CPU {
    let mut program = [0u8; MEM_SIZE];
    let len = EMULATOR.len();
    let clen = chip8.len().min(4096);

    program[20..100].copy_from_slice(&CHIP_FONTSET);
    program[532..532 + clen].copy_from_slice(chip8);

    program[6196..6196 + len].copy_from_slice(&EMULATOR);
    program[0..=6].copy_from_slice(&[2, 4, 6, 8, 10, 12, 14]);
    program[I + 1] = 0x14;

    CPU::new_from_start(program, START)
}

fn draw(window: &mut Window, display: &[u8]) -> Result<(), Error> {
    let mut buffer = vec![0; (WIDTH * SCALE) * (HEIGHT * SCALE)];

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let color = if display[y * WIDTH + x] == 1 {
                0xFFFFFFFF
            } else {
                0x00000000
            };

            for dy in 0..SCALE {
                for dx in 0..SCALE {
                    let sx = x * SCALE + dx;
                    let sy = y * SCALE + dy;
                    buffer[sy * WIDTH * SCALE + sx] = color;
                }
            }
        }
    }

    window.update_with_buffer(&buffer, WIDTH * SCALE, HEIGHT * SCALE)?;

    Ok(())
}

fn map_key(k: Key) -> Option<u8> {
    match k {
        Key::Key1 => Some(0x1),
        Key::Key2 => Some(0x2),
        Key::Key3 => Some(0x3),
        Key::Key4 => Some(0xC),
        Key::Q => Some(0x4),
        Key::W => Some(0x5),
        Key::E => Some(0x6),
        Key::R => Some(0xD),
        Key::A => Some(0x7),
        Key::S => Some(0x8),
        Key::D => Some(0x9),
        Key::F => Some(0xE),
        Key::Z => Some(0xA),
        Key::X => Some(0x0),
        Key::C => Some(0xB),
        Key::V => Some(0xF),
        _ => None,
    }
}

fn debug_chip(cpu: &CPU) {
    let mem = cpu.memory();
    let mut registers = String::new();
    for (idx, reg) in mem[0..16].iter().enumerate() {
        registers.push_str(&format!("Reg {idx}: 0x{reg:02x}\n"));
    }
    println!("{registers}");

    println!("I: 0x{:02x}{:02x}", mem[16], mem[17]);

    println!("PC: 0x{:02x}{:02x}", cpu.register(4), cpu.register(5));
    println!("Stack pointer: 0x{:02x}", mem[18]);
    println!("Key flag: 0x{:02x}", mem[19]);
}

struct Chip {
    unknown: [u8; 2],
    unknown_curr: u8,
    draw: bool,
    sound: bool,
    keys: [u8; 16],
    /// The key read requested by the cpu.
    read_key: Option<u8>,
    keypress: u8,
    delay_timer: u8,
    sound_timer: u8,
    draw_px: (u8, u8, u8),
    end_draw: u8,
    pixels: Vec<(usize, u8)>,
    px_idx: Option<usize>,
    last_update: Option<Instant>,
}

impl Chip {
    fn new() -> Self {
        Self {
            unknown: [0, 0],
            unknown_curr: 0,
            keys: [0x0; 16],
            read_key: None,
            keypress: 0xff,
            draw: false,
            sound: false,
            delay_timer: 0,
            sound_timer: 0,
            draw_px: (0, 0, 0),
            end_draw: 0x01,
            pixels: vec![],
            px_idx: None,
            last_update: None,
        }
    }

    fn step(&mut self) {
        if let Some(last_update) = self.last_update.as_ref() {
            if last_update.elapsed() >= Duration::from_micros(4_500) {
                self.delay_timer = self.delay_timer.saturating_sub(1);
                self.sound_timer = self.sound_timer.saturating_sub(1);

                if self.sound_timer == 0 {
                    self.sound = false
                } else {
                    self.sound = true
                }

                self.last_update = Some(Instant::now());
            }
        } else {
            self.last_update = Some(Instant::now());
        }
    }

    fn set_key(&mut self, key: u8, pressed: bool) {
        if pressed {
            self.keypress = key;
        } else if key == self.keypress {
            self.keypress = 0xff
        }

        self.keys[key as usize] = u8::from(pressed);
    }

    fn draw(&mut self, memory: &[u8]) {
        let x = self.draw_px.0 as usize % WIDTH;
        let y = self.draw_px.1 as usize % HEIGHT;
        let height = self.draw_px.2 as usize;
        let i = ((memory[I] as usize) << 8) | (memory[I + 1]) as usize;
        self.pixels.clear();

        for row in 0..height {
            let pixel = memory[i + row];

            for bit_offset in 0..8 {
                if (x + bit_offset) < WIDTH && (y + row) < HEIGHT {
                    let x = (x + bit_offset) % WIDTH;
                    let y = (y + row) % HEIGHT;
                    let pos = x + (y * WIDTH);
                    let pos = GFX + pos;
                    if (pixel & (0x80 >> bit_offset)) != 0 {
                        let clip = memory[pos] == 1;

                        self.pixels.push((pos, clip as u8));
                    }
                }
            }
        }

        if !self.pixels.is_empty() {
            self.px_idx = Some(0);
            self.end_draw = 0;
        } else {
            self.px_idx = None;
            self.end_draw = 1;
        }
    }
}

impl Bus for Chip {
    fn read(&mut self, _cpu: &CPU, port: u8) -> u8 {
        match port {
            0x01 => rand_u8(),
            0x02 => match self.read_key.take() {
                Some(key) => self.keys[key as usize],
                None => 0x00,
            },
            0x03 => self.delay_timer,
            0x04 => self.sound_timer,
            0x05 => self.keypress,
            0x07 => self.end_draw,
            0x08 => {
                let idx = self.px_idx.expect("Read Clip called incorrectly");
                let clip = self.pixels.get(idx).expect("Incorrect Clip index").1;

                if self.pixels.len() <= idx + 1 {
                    self.px_idx = None;
                    self.end_draw = 1;
                } else {
                    self.px_idx = Some(idx + 1);
                }

                clip
            }
            0x09 => {
                let idx = self.px_idx.expect("Read lsb Pos called incorrectly");
                let pos = self.pixels.get(idx).expect("Incorrect Pos index").0;

                let lsb = (pos & 255) as u8;
                lsb
            }
            0x0a => {
                let idx = self.px_idx.expect("Read msb Pos called incorrectly");
                let pos = self.pixels.get(idx).expect("Incorrect Pos index").0;

                let msb = (pos >> 8) as u8;

                msb
            }
            unknown => {
                panic!("Unreacheable read at port: 0x{unknown:02x}")
            }
        }
    }

    fn write(&mut self, cpu: &CPU, port: u8, data: u8) {
        match port {
            0x01 => {
                self.unknown[self.unknown_curr as usize] = data;
                if self.unknown_curr == 1 {
                    println!(
                        "Unknown opcode: 0x{:02x}{:02x}",
                        self.unknown[0], self.unknown[1]
                    );
                }
                self.unknown_curr = 1 - self.unknown_curr;
            }
            0x02 => {
                self.read_key = Some(data);
            }
            0x03 => {
                self.delay_timer = data;
            }
            0x04 => {
                self.sound_timer = data;
            }
            0x05 => {
                self.draw_px.0 = data;
            }
            0x06 => {
                self.draw_px.1 = data;
            }
            0x07 => {
                self.draw_px.2 = data;
                self.draw(cpu.memory());
            }
            0x08 => {
                self.draw = data == 0x01;
            }
            unknown => {
                panic!("Unreacheable write at port: 0x{unknown:02x}")
            }
        }
    }
}

fn rand_u8() -> u8 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();

    (nanos & 0xFF) as u8
}

#[derive(Debug)]
pub enum Error {
    File(std::io::Error),
    AudioDevice(rodio::DevicesError),
    Decoder(rodio::decoder::DecoderError),
    Play(rodio::PlayError),
    Stream(rodio::StreamError),
    IO(std::io::Error),
    Mini(minifb::Error),
}

impl From<rodio::DevicesError> for Error {
    fn from(value: rodio::DevicesError) -> Self {
        Self::AudioDevice(value)
    }
}

impl From<rodio::PlayError> for Error {
    fn from(value: rodio::PlayError) -> Self {
        Self::Play(value)
    }
}

impl From<rodio::StreamError> for Error {
    fn from(value: rodio::StreamError) -> Self {
        Self::Stream(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::File(value)
    }
}

impl From<rodio::decoder::DecoderError> for Error {
    fn from(value: rodio::decoder::DecoderError) -> Self {
        Self::Decoder(value)
    }
}

impl From<minifb::Error> for Error {
    fn from(value: minifb::Error) -> Self {
        Self::Mini(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AudioDevice(error) => error.fmt(f),
            Self::Play(error) => error.fmt(f),
            Self::Stream(error) => error.fmt(f),
            Self::File(error) => error.fmt(f),
            Self::Decoder(error) => error.fmt(f),
            Self::IO(error) => error.fmt(f),
            Self::Mini(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AudioDevice(error) => Some(error),
            Self::Play(error) => Some(error),
            Self::Stream(error) => Some(error),
            Self::File(error) => Some(error),
            Self::Decoder(error) => Some(error),
            Self::IO(error) => Some(error),
            Self::Mini(error) => Some(error),
        }
    }
}

// let _ = write("./programs/chip8/chip8", PROGRAM);
// println!("Done writing");

// let program = include_bytes!("../chip8");
// let program = include_bytes!("../../../games/invaders/invaders/invaders.h");

// dbg!(program.len());
// for i in program {
//     println!("{i:02x}");
// }

// println!("{program:?}");
