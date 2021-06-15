#![no_main]
#![no_std]

use ramloader as _; // global logger + panicking-behavior + memory layout

const KNOWN_ADDRESS: usize = 0x2002_0000;

static TEXT: &[u8] = include_bytes!("../text.bin");
static VECTOR_TABLE: &[u8] = include_bytes!("../vector-table.bin");

#[cortex_m_rt::entry]
fn main() -> ! {
    let periph = cortex_m::Peripherals::take().unwrap();

    unsafe {
        // write .vector_table to RAM 0x2002_0000
        {
            let src = VECTOR_TABLE.as_ptr();
            let dst = KNOWN_ADDRESS as *mut u8;
            let len = VECTOR_TABLE.len();

            defmt::info!(".vector_table");
            defmt::dbg!(src, dst, len);
            core::ptr::copy_nonoverlapping(
                src,
                dst,
                len,
            );
        }

        {
            // write .text to RAM 0x2002_0100
            let src = TEXT.as_ptr();
            let dst = (KNOWN_ADDRESS + VECTOR_TABLE.len()) as *mut u8;
            let len = TEXT.len();

            defmt::info!(".text");
            defmt::dbg!(src, dst, len);
            core::ptr::copy_nonoverlapping(
                src,
                dst,
                len,
            );
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

    // defmt::info!("did not crash");
    // ramloader::exit()
}
