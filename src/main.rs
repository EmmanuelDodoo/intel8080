use intel8080::*;
use std::fs::read;

fn main() {
    let mut args = std::env::args();
    let _ = args.next();

    let mode = args.next();

    if mode == Some("--tests".to_owned()) {
        run_tests();
        return;
    }
}

fn run_tests() {
    println!("Running tests");
    test("8080PRE");
    test("TST8080");
    test("CPUTEST");
    test("8080EXM");
}

fn test(test: &str) {
    fn test_prep(program: &[u8]) -> Emulator<'_> {
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

    let rom = read(format!("./tests/{test}.COM")).unwrap();
    let mut emulator = test_prep(&rom);

    println!("\n**** Testing {test}.COM");

    let mut ins = 0usize;

    while emulator.testing() {
        ins += 1;
        emulator.cycle();
    }

    println!("\n**** {ins} instructions");
}
