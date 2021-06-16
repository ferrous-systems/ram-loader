#![no_main]
#![no_std]

use nrf52840_hal::{gpio, uarte, Uarte};
use ramloader as _; // global logger + panicking-behavior + memory layout

#[cortex_m_rt::entry]
fn main() -> ! {
    // let core_periphals = cortex_m::Peripherals::take().unwrap();
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
                // cts: Some(p0.p0_07.into_floating_input().degrade()),
                // rts: Some(p0.p0_05.into_push_pull_output(gpio::Level::High).degrade()),
            },
        )
    };

    let mut uarte = Uarte::new(
        uart0,
        cdc_pins,
        uarte::Parity::EXCLUDED,
        uarte::Baudrate::BAUD115200,
    );

    let mut request_buffer = [0];

    // defmt::info!("did not crash");
    // ramloader::exit()
    loop {
        defmt::info!("blocking read");
        uarte.read(&mut request_buffer).unwrap();

        defmt::dbg!(request_buffer);

        uarte.write(&request_buffer).unwrap();
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
