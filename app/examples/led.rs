//! LED blinks if interrupts are working

#![no_std]
#![no_main]

use nrf52840_hal::{
    self as hal,
    gpio::{p0, Level},
    prelude::*,
};
use panic_halt as _;

use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    let periph = hal::pac::Peripherals::take().unwrap();
    let pins = p0::Parts::new(periph.P0);
    let mut led = pins.p0_13.degrade().into_push_pull_output(Level::High);

    // LED on
    led.set_low().ok();

    loop {
        cortex_m::asm::nop();
    }
}
