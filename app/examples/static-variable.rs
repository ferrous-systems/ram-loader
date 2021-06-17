//! LED turns on if static variables are correctly initialized

#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU32, Ordering};

use nrf52840_hal::{
    self as hal,
    gpio::{p0, Level},
    prelude::*,
};
use panic_halt as _;

use cortex_m_rt::entry;

const KEY: u32 = 0xabcd_0123;
static LOCK: AtomicU32 = AtomicU32::new(KEY);

#[entry]
fn main() -> ! {
    let periph = hal::pac::Peripherals::take().unwrap();
    let pins = p0::Parts::new(periph.P0);
    let mut led = pins.p0_13.degrade().into_push_pull_output(Level::High);

    // if variable was initialized with the expected bit pattern
    if LOCK.load(Ordering::Relaxed) == KEY {
        // turn on LED
        led.set_low().ok();
    }

    loop {
        cortex_m::asm::nop();
    }
}

// interrupt handler that will not be called
#[cortex_m_rt::exception]
fn SysTick() {
    // prevent the compiler from optimizing away the static variable and *always* turning on the LED
    LOCK.fetch_add(1, Ordering::Relaxed);
}
