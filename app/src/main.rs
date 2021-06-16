#![no_std]
#![no_main]

use core::{
    cell::RefCell,
    sync::atomic::{AtomicU32, Ordering},
};

use cortex_m::interrupt::{self, Mutex};
use nrf52840_hal::{
    self as hal,
    gpio::{p0, Level},
    prelude::*,
};
use panic_halt as _;

use cortex_m_rt::entry;

// .data
static VARIABLE: AtomicU32 = AtomicU32::new(1);

static MUTEX: Mutex<RefCell<Option<hal::gpio::Pin<hal::gpio::Output<hal::gpio::PushPull>>>>> =
    Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let core_peripherals = cortex_m::Peripherals::take().unwrap();
    let periph = hal::pac::Peripherals::take().unwrap();
    let pins = p0::Parts::new(periph.P0);
    let mut led = pins.p0_13.degrade().into_push_pull_output(Level::High);

    let mut syst = core_peripherals.SYST;
    syst.set_reload(16_000_000);

    // LED on
    led.set_low().ok();

    if VARIABLE.load(Ordering::Relaxed) == 1 {
        interrupt::free(|cs| {
            let pin_guard = MUTEX.borrow(cs);
            *pin_guard.borrow_mut() = Some(led);
        });
        syst.enable_interrupt();
        syst.enable_counter();
    }

    loop {
        // your code goes here
    }
}

#[cortex_m_rt::exception]
fn SysTick() {
    interrupt::free(|cs| {
        let mut led_guard = MUTEX.borrow(cs).borrow_mut();
        let led = led_guard.as_mut().unwrap();
        if led.is_set_low().unwrap() {
            led.set_high().unwrap()
        } else {
            led.set_low().unwrap()
        }
    });
}
