# Intel 8080 Emulator

An emulator for the Intel 8080 processor.

## Features

- [x] Complete and accurate emulation of instruction set.

- [x] Support for external I/O handling

- [x] Interrupt handling


## Running tests

You can run the tests by running `cargo run -- --tests`. The emulator passes the following tests:

- [x] 8080PRE.COM
- [x] TST8080.COM
- [x] CPUTEST.COM
- [x] 8080EXM.COM

The standard output is as follows:

```
**** Testing 8080PRE.COM
    8080 Preliminary tests complete
**** 169681 instructions

**** Testing TST8080.COM
    MICROCOSM ASSOCIATES 8080/8085 CPU DIAGNOSTIC
    VERSION 1.0  (C) 1980

    CPU IS OPERATIONAL
**** 95070 instructions

**** Testing CPUTEST.COM

    DIAGNOSTICS II V1.2 - CPU TEST
    COPYRIGHT (C) 1981 - SUPERSOFT ASSOCIATES

    ABCDEFGHIJKLMNOPQRSTUVWXYZ
    CPU IS 8080/8085
    BEGIN TIMING TEST
    END TIMING TEST
    CPU TESTS OK

**** 5528396777 instructions 

**** Testing 8080EXM.COM
    8080 instruction exerciser
    dad <b,d,h,sp>................  PASS! crc is:14474ba6
    aluop nn......................  PASS! crc is:9e922f9e
    aluop <b,c,d,e,h,l,m,a>.......  PASS! crc is:cf762c86
    <daa,cma,stc,cmc>.............  PASS! crc is:bb3f030c
    <inr,dcr> a...................  PASS! crc is:adb6460e
    <inr,dcr> b...................  PASS! crc is:83ed1345
    <inx,dcx> b...................  PASS! crc is:f79287cd
    <inr,dcr> c...................  PASS! crc is:e5f6721b
    <inr,dcr> d...................  PASS! crc is:15b5579a
    <inx,dcx> d...................  PASS! crc is:7f4e2501
    <inr,dcr> e...................  PASS! crc is:cf2ab396
    <inr,dcr> h...................  PASS! crc is:12b2952c
    <inx,dcx> h...................  PASS! crc is:9f2b23c0
    <inr,dcr> l...................  PASS! crc is:ff57d356
    <inr,dcr> m...................  PASS! crc is:92e963bd
    <inx,dcx> sp..................  PASS! crc is:d5702fab
    lhld nnnn.....................  PASS! crc is:a9c3d5cb
    shld nnnn.....................  PASS! crc is:e8864f26
    lxi <b,d,h,sp>,nnnn...........  PASS! crc is:fcf46e12
    ldax <b,d>....................  PASS! crc is:2b821d5f
    mvi <b,c,d,e,h,l,m,a>,nn......  PASS! crc is:eaa72044
    mov <bcdehla>,<bcdehla>.......  PASS! crc is:10b58cee
    sta nnnn / lda nnnn...........  PASS! crc is:ed57af72
    <rlc,rrc,ral,rar>.............  PASS! crc is:e0d89235
    stax <b,d>....................  PASS! crc is:2b0471e9
    Tests complete
**** 2919050698 instructions

```

## Trivial Program
I included a trivial i8080 program which echoes 1 byte from stdin to stdout. Run
this with `cargo run -- --trivial`.

## Games

The project also contains [implementations of games made for the i8080](games/README.md). It only 
contains Space Invaders currently, but I will add more games later. 


## Acknowledgements

Much thanks to 

- [superzazu's own i8080 emulator](https://github.com/superzazu/8080/) for both the tests and solutions for niche
  edge cases.

## Resources

- [Processor Manual](https://drakeor.com/uploads/8080-Programmers-Manual.pdf)

- [Intel 8080 instruction set](https://pastraiser.com/cpu/i8080/i8080_opcodes.html)

- [Manuals for the tests](https://altairclone.com/downloads/cpu_tests/8080_8085%20CPU%20Exerciser.pdf)

- [Computer Archeology for Space Invaders documentation](https://www.computerarcheology.com/Arcade/SpaceInvaders/Hardware.html)

- [Audio from Classic Gaming](https://www.classicgaming.cc/classics/space-invaders/sounds)

