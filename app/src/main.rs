#![no_std]
#![no_main]

use panic_halt as _; 
use nrf52840_hal::{self  as hal, gpio::{p0, Level}, prelude::*};

use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    let periph = hal::pac::Peripherals::take().unwrap();
    let pins = p0::Parts::new(periph.P0);
    let mut led = pins.p0_13.degrade().into_push_pull_output(Level::High);

    led.set_low().ok();

    loop {
        // your code goes here
    }
}
