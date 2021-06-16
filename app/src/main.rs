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

// .data
static VARIABLE_IN_DOT_DATA: AtomicU32 = AtomicU32::new(1);
static VARIABLE_IN_DOT_DATA2: AtomicU32 = AtomicU32::new(1);

// .rodata
static VARIABLE_IN_DOT_RODATA: &str = "Hello, world";

#[entry]
fn main() -> ! {
    let periph = hal::pac::Peripherals::take().unwrap();
    let pins = p0::Parts::new(periph.P0);
    let mut led = pins.p0_13.degrade().into_push_pull_output(Level::High);

    led.set_low().ok();

    unsafe {
        VARIABLE_IN_DOT_RODATA.as_bytes().as_ptr().read_volatile();
    }

    loop {
        VARIABLE_IN_DOT_DATA.fetch_add(1, Ordering::Relaxed);
        VARIABLE_IN_DOT_DATA2.fetch_add(1, Ordering::Relaxed);
        // your code goes here
    }
}
