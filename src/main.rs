use intel8080::*;
use std::fs::read;
use std::io::{self, Write};

fn main() {
    let mut args = std::env::args();
    let _ = args.next();

    let mode = args.next();

    match mode.as_ref() {
        Some(val) if val == "--tests" => run_tests(),
        Some(val) if val == "--trivial" => trivial(),
        _ => {}
    }
}

fn _temp_main() {
    let mut memory = [0; MEM_SIZE];
    #[rustfmt::skip]
    let program = [
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0x3e,
        0xaa,
        0x32,
        0x21,
        0x00,
        0xc9,
        
        0x3e,
        0x00,
        0xfb,
        0x00,
        0x00,
        0x3e,
        0x55,
        0x32,
        0x21,
        0x01,
        0x76,
    ];
    let len = program.len().min(MEM_SIZE);

    memory[..len].copy_from_slice(&program[..len]);

    let mut cpu = CPU::new_from_start(memory, 14);
    println!("Program loaded\n");

    loop {
        print!("> ");

        io::stdout().flush().unwrap();

        let mut input = String::new();

        if let Err(error) = io::stdin().read_line(&mut input) {
            eprintln!("{error}");
            continue;
        }

        let mut input = input.split_whitespace();
        let command = input.next();

        match command {
            Some("i") => {
                cpu.interrupt(0xcf);
                cpu.debug();
            }
            _ => {
                cpu.cycle(&mut ());
                cpu.debug();
                println!("0x{:04x}", cpu.memory()[0x0021]);
                println!("0x{:04x}", cpu.memory()[0x0121]);
                continue;
            }
        }
    }
}

/// A [`Bus`] which reads from and writes to `std::io`.
struct Trivial;

impl Bus for Trivial {
    fn read(&mut self, _cpu: &CPU, _port: u8) -> u8 {
        //println!("Reading from port {port}.");
        let mut input = String::new();

        io::stdin().read_line(&mut input).unwrap();

        let input = input.trim().as_bytes();

        if !input.is_empty() { input[0] } else { 0x00 }
    }

    fn write(&mut self, _cpu: &CPU, _port: u8, data: u8) {
        println!("{}", data as char);
        //println!("Writing {} to port {port}", data as char);
    }
}

pub fn trivial() {
    #[rustfmt::skip]
    let program = [
        0x06, // move 0xff to b
        0xff,
        0xdb, // read input from port 0
        0x00,
        0x4f, // move acc to c
        0x80, // add b to acc
        0xda, // jump if carry
        0x0c,
        0x00,
        0xc3, // jump back to read input
        0x02,
        0x00,
        0x79, // move c to acc
        0xd3, // output acc to port 0
        0x00,
        0x3e, // set acc to 0
        0x00,
        0xc3, // jump back to read input
        0x02,
        0x00,
    ];

    let mut cpu = CPU::new(&program);
    println!("Program loaded\n");

    let mut bus = Trivial;

    while !cpu.halted() {
        cpu.cycle(&mut bus);
    }

    println!("\nProgram halted");
}

pub fn run_tests() {
    println!("Running tests");
    test("8080PRE");
    test("TST8080");
    test("CPUTEST");
    test("8080EXM");
}

fn test(test: &str) {
    fn test_prep(program: &[u8]) -> CPU {
        let start = 0x100;
        let mut memory = [0; MEM_SIZE];
        let len = program.len().min(MEM_SIZE - start);

        memory[start..start + len].copy_from_slice(&program[..len]);

        memory[0x0000] = 0xd3;
        memory[0x0001] = 0x00;

        memory[0x0005] = 0xd3;
        memory[0x0006] = 0x01;
        memory[0x0007] = 0xc9;

        CPU::new_from_start(memory, start as u16)
    }

    let rom = read(format!("./tests/{test}.COM")).unwrap();
    let mut emulator = test_prep(&rom);

    println!("\n**** Testing {test}.COM");

    let mut ins = 0usize;
    let mut bus = TestingBus::new();

    while !bus.exit {
        ins += 1;
        emulator.cycle(&mut bus);
    }

    println!("\n**** {ins} instructions");
}

#[derive(Default)]
pub struct TestingBus {
    pub exit: bool,
}

impl TestingBus {
    pub fn new() -> Self {
        TestingBus { exit: false }
    }
}

impl Bus for TestingBus {
    fn read(&mut self, _cpu: &CPU, _port: u8) -> u8 {
        0x00
    }

    fn write(&mut self, cpu: &CPU, port: u8, _data: u8) {
        if port == 0 {
            self.exit = true;
        } else if port == 1 {
            let operation = cpu.register(1);

            if operation == 2 {
                let e = cpu.register(3);
                print!("{}", e as char);
            } else {
                let mut addr = ((cpu.register(2) as u16) << 8) | (cpu.register(3) as u16);
                print!("{}", cpu.memory()[addr as usize] as char);

                while cpu.memory()[addr as usize] != 36 {
                    print!("{}", cpu.memory()[addr as usize] as char);
                    addr += 1;
                }
            }
        }
    }
}
