#![no_main]
#![no_std]

use ramloader as _; // global logger + panicking-behavior + memory layout

const KNOWN_ADDRESS: usize = 0x2000_0000 + 0x20000;

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello, world!");

    // write to VTOR
    // ???
    // make it work

    ramloader::exit()
}
