# RAM loader project

```
$ tree .
.
  bootloader
    memory.x
  app
    memory.x
```

## Firmware bootloader (goes into Flash)

## memory layout

- normal cortex-m-rt program

- this will use RAM
- it has to write to RAM
- it should not write to its static variables or stack
- thus we need to split RAM between
  - for use of the bootloader
  - for use of the target application

``` text
+------+ REAL_RAM_END
|      | (.bss+.data+stack) "RAM"
| RAM2 | - SPLIT_ADDRESS
|      | (.text + .rodata) "FLASH"
+------+ KNOWN_ADDRESS
| RAM1 |
+------+ REAL_RAM_START
```

- `RAM1` is for FW ram loader

- `memory.x`
  - `RAM.ORIGIN` = `REAL_RAM_START`
  - `RAM.LENGTH` = `KNOWN_ADDRESS` - `REAL_RAM_START`

## behavior

- set up a serial port
- wait for messages from the host
- on incoming message
  - verify the message integrity
    - chunking = ihex Data::Record frames
  - verify memory address
    - if wrong,
      - 1. ignore all other frames; do not write to memory; reboot? 
  - launch the application loaded into RAM
    - update the `VTOR` register with RAM location of vector table
    - change the SP register

- approach 1
``` rust
let vector_table: &[u32; 128] = /* .. */;
VTOR.write(vector_table.as_ptr());
let initial_sp = vector_table[0];
let reset_handler = vector_table[1];
// this part requires assembly 
SP = initial_sp;
reset_handler()
```

- approach 2 <- try out this without any host communication?
  - write VTOR
  - trigger a software reset using some SCB register

## App project skeleton 

- tweaked MEMORY.x to put program in RAM

- `RAM` region is `memory.x`
  - `RAM.ORIGIN` = `KNOWN_ADDRESS`
  - `RAM.LENGTH` = `REAL_RAM_END` - `KNOWN_ADDRESS`

- `cortex-m-rt` does not support putting `.text`, `.rodata` into `RAM`

- we can lie in the `memory.x`

``` text
MEMORY {
    FLASH : ORIGIN = $KNOWN_ADDRESS, LENGTH = $SPLIT_ADDRESS - $KNOWN_ADDRESS
    RAM : ORIGIN = $SPLIT_ADDRESS, LENGTH = REAL_RAM_END - $SPLIT_ADDRESS
}
```

- verify that .text, .rodata are where they should be
- verify that .vector_table is located at `KNOWN_ADDRESS`

- add `xtask` to turn ELF into `.hex` file using `rust-objcopy`

## Host command line tool to send programs

- use serial port (`/dev/tty*`), the one provided by the probe on the DK dev board
- input: ELF file or .hex file
- communication: maybe postcard + COBS
- USAGE: `program path/to/file.hex`
  - arguments
    - ELF file (program to flash)
    - where to put the program in RAM is going to be `KNOWN_ADDRESS`
  - operations:
    - write bytes to RAM
    - start program with vector table at `address`
- architecture: server client comm

- what needs to be sent 
  - input: ELF (option 2)
    - all "loadable" sections (`object` crate has API for this)
  - input: ihex format (`.hex`) (option 1)
    - `objcopy -f hex` to go from ELF to ihex
    - self-describing format
    - frames: [START LENGTH BYTES; _]
    - there's a crate to parse ihex files (see training material)

- how to sent it
  - maintain ihex Data::Recard format
  - but include a CRC (just in case?)

## Things to keep in mind

- `KNOWN_ADDRESS` needs to be aligned to `N`
  - at the start of `KNOWN_ADDRESS` we are going to put the `.vector_table`
  - `VTOR` has an alignment requirement

## Initial decisions

- KNOWN_ADDRESS is half the RAM address space
  - REAL_RAM_START = 0x2000_0000
  - KNOWN_ADDRESS = 0x2000_0000 + 128 KB
  - REAL_RAM_END = 0x2000_0000 + 256 KB
- nice to have: try to reduce the footprint of the RAM loader

## Communication protocol

- postcard + COBS 

``` rust
enum Host2TargetMessage {
    // StartOfFile { number_of_frames: u16 },
    // maps to a single ihex::Record::Data
    Data { offset: u16, bytes: &[u8] }
    EndOfFile,
}

enum Target2HostMessage {
    DataWritten { offset: u16 },
    // Data frame outside valid RAM address space
    Abort,
}
```

## Minimal implemantion

- *assume everything will go perfectly*
- no CRC checks on target-hosh comm
- no timeout check on target
- but add notes about things to keep in mind when making this production-ready

## Implementation order

### FW - ramloader

- [x] check that we can launch a RAM program w/o any serial port communication with the host
  - [x] ~~try approach 2 in bootloader.behavior~~
  - [x] otherwise, try approach 1 -> `cortex_m::asm::bootload` :thumbsup:

- [x] implement serial communication
  - [x] postcard + COBS
- ~~receive Record::Data frames, write those into RAM, continue with `bootload`~~
- [x] handle `Write` command
- [x] handle `Execute` command

- nice to have + cleanups

### App

- [x] start from `cortex-m-quickstart` template
- [x] start by *not* using `defmt`
- nice to have: try to use `defmt`
- nice to have: `xtask` to turn ELF into .hex
- [x] double-check: we can fake "FLASH" which will actually be in real HW RAM 

### Host tool

- [x] requires: ELF located in RAM (`app`)
- ~~turn ELF into `.hex` using `rust-objcopy` (manually)~~
- ~~use the `ihex` crate to make sure we can read out the data (`Record::Data`)~~
  - ~~verify that `Record::Data.offset` is absolute RAM address~~

- [x] set up postcard + COBS
- ~~take .hex file as argument; parse that into Record::Data frames; send those to target~~

- [x] take ELF file as argument
- [x] extract loadable data from ELF
- [x] chunk the data
- [x] send the chunks over serial port
- [x] send Execute command
- [x] it worked! :tada:

- nice to have + cleanups

## Thing to test

- `app` lications
  - interrupts work (if they don't work = didn't set VTOR)
  - `.data` variables work
    - these are initialized on startup by `cortex-m-rt`
    - initial values are in Flash (normally), at Load Memory Address (not Virtual Memory Address)

## TODOs

- create blanks, skeleton project, instructions
- NOTE that board must be reset to flash a new program

## Nice to have & cleanups

- serial communication on Windows is pretty slow: 1s per exchanged/roundtrip serial port message
- fix bug in flip-link and use flip-link in `ram-loader` FW
- make `payload_size` larger
- use `defmt` in `app` FW
- refactor `ram-loader` `main` into subfunctions
- LED indicator that ramloader is waiting for a new program
- `elfloader` better argument handling (maybe use `clap` / `structopt`)- add explanatory comments to the code