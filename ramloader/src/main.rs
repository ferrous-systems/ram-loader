#![no_main]
#![no_std]

use core::ops::Range;

use common::{Host2TargetMessage, Target2HostMessage};
use heapless::Vec;
use nrf52840_hal::{gpio, uarte, Uarte};
use ramloader as _; // global logger + panicking-behavior + memory layout

const RAM_PROGRAM_START_ADDRESS: u32 = 0x2002_0000;
const RAM_PROGRAM_END_ADDRESS: u32 = 0x2004_0000;
const VALID_RAM_PROGRAM_ADDRESS: Range<u32> = RAM_PROGRAM_START_ADDRESS..RAM_PROGRAM_END_ADDRESS;

#[cortex_m_rt::entry]
fn main() -> ! {
    let core_periphals = cortex_m::Peripherals::take().unwrap();
    let p = nrf52840_hal::pac::Peripherals::take().unwrap();

    let (uart0, cdc_pins) = {
        let p0 = gpio::p0::Parts::new(p.P0);
        (
            p.UARTE0,
            uarte::Pins {
                txd: p0.p0_06.into_push_pull_output(gpio::Level::High).degrade(),
                rxd: p0.p0_08.into_floating_input().degrade(),
                cts: None,
                rts: None,
            },
        )
    };

    let mut uarte = Uarte::new(
        uart0,
        cdc_pins,
        uarte::Parity::EXCLUDED,
        uarte::Baudrate::BAUD115200,
    );

    let mut serial_rx_buffer = [0; 1];

    let mut cobs_buffer = Vec::<_, { common::POSTCARD_BUFFER_SIZE }>::new();
    defmt::info!("ready");
    loop {
        // defmt::info!("blocking single-byte read");
        uarte.read(&mut serial_rx_buffer).unwrap();
        let byte = serial_rx_buffer[0];
        cobs_buffer.push(byte).unwrap();

        if byte == common::COBS_DELIMITER {
            // if byte == common::COBS_DELIMITER {
            defmt::info!("found delimiter");
            defmt::dbg!(&*cobs_buffer);

            let host2target_message: Host2TargetMessage =
                postcard::from_bytes_cobs(&mut cobs_buffer).unwrap();
            defmt::dbg!(&host2target_message);

            let response = match host2target_message {
                Host2TargetMessage::Ping => Target2HostMessage::Pong,

                Host2TargetMessage::Write {
                    start_address,
                    data,
                } => {
                    if VALID_RAM_PROGRAM_ADDRESS.contains(&start_address) {
                        let src = data.as_ptr();
                        let dst = start_address as usize as *mut u8;
                        let len = data.len();

                        defmt::dbg!(src, dst, len);
                        unsafe {
                            core::ptr::copy_nonoverlapping(src, dst, len);
                        }
                        Target2HostMessage::WriteOk
                    } else {
                        defmt::error!("address `{}` is invalid", start_address);
                        Target2HostMessage::InvalidAddress
                    }
                }

                Host2TargetMessage::Execute => {
                    // write to VTOR
                    unsafe {
                        core_periphals.SCB.vtor.write(RAM_PROGRAM_START_ADDRESS);
                    }

                    // flush defmt messages
                    cortex_m::asm::delay(1_000_000);

                    unsafe { cortex_m::asm::bootload(RAM_PROGRAM_START_ADDRESS as *const u32) }
                }
            };

            let response_bytes = postcard::to_vec_cobs::<_, 256>(&response).unwrap();

            defmt::dbg!(&*response_bytes);

            uarte.write(&response_bytes).unwrap();
            cobs_buffer.clear();
        }
    }
}

#[allow(dead_code)]
fn launch_program(periph: cortex_m::Peripherals) {
    const KNOWN_ADDRESS: usize = 0x2002_0000;

    // .text
    static TEXT: &[u8] = include_bytes!("../text.bin");
    // .vector_table
    static VECTOR_TABLE: &[u8] = include_bytes!("../vector-table.bin");

    unsafe {
        // write .vector_table to RAM 0x2002_0000
        {
            let src = VECTOR_TABLE.as_ptr();
            let dst = KNOWN_ADDRESS as *mut u8;
            let len = VECTOR_TABLE.len();

            defmt::info!(".vector_table");
            defmt::dbg!(src, dst, len);
            core::ptr::copy_nonoverlapping(src, dst, len);
        }

        {
            // write .text to RAM 0x2002_0100
            let src = TEXT.as_ptr();
            let dst = (KNOWN_ADDRESS + VECTOR_TABLE.len()) as *mut u8;
            let len = TEXT.len();

            defmt::info!(".text");
            defmt::dbg!(src, dst, len);
            core::ptr::copy_nonoverlapping(src, dst, len);
        }

        // write to VTOR
        periph.SCB.vtor.write(KNOWN_ADDRESS as u32);

        cortex_m::asm::delay(1_000_000);

        // # launch the program
        // approach 1 failed
        // cortex_m::peripheral::SCB::sys_reset()

        cortex_m::asm::bootload(KNOWN_ADDRESS as *const u32)
        // // approach 2
        // // 1st entry in vector table
        // let initial_sp = (KNOWN_ADDRESS as *const u32).read();
        // // 2nd entry in vector table
        // let reset_handler = (KNOWN_ADDRESS as *const u32).offset(1).read();

        // defmt::info!("initial_sp={:x}", initial_sp);
        // defmt::info!("reset_handler={:x}", reset_handler);

        // // write to SP
        // // call reset_handler
        // cortex_m::asm::bootstrap(initial_sp as *const u32, reset_handler as *const u32)
    }
}
