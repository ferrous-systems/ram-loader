#![no_main]
#![no_std]

use core::ops::{Range, RangeInclusive};

use common::{Host2TargetMessage, Target2HostMessage};
use cortex_m::peripheral::SCB;
use heapless::Vec;
use nrf52840_hal::{
    gpio::{
        self,
        p0::{self, P0_06, P0_08, P0_14, P0_15, P0_16},
        Disconnected, Level, Output, PushPull,
    },
    pac::UARTE0,
    prelude::*,
    uarte, Uarte,
};
use ramloader as _; // global logger + panicking-behavior + memory layout

const RAM_PROGRAM_START_ADDRESS: u32 = 0x2002_0000;
const RAM_PROGRAM_END_ADDRESS: u32 = 0x2004_0000;
const VALID_RAM_PROGRAM_ADDRESS: RangeInclusive<u32> =
    RAM_PROGRAM_START_ADDRESS..=RAM_PROGRAM_END_ADDRESS;

#[cortex_m_rt::entry]
fn main() -> ! {
    let core_peripherals = cortex_m::Peripherals::take().unwrap();
    let nrf_peripherals = nrf52840_hal::pac::Peripherals::take().unwrap();

    let port0_pins = p0::Parts::new(nrf_peripherals.P0);
    let mut leds = Leds::on(port0_pins.p0_14, port0_pins.p0_15, port0_pins.p0_16);

    let mut serial_port =
        initialize_serial_port(port0_pins.p0_06, port0_pins.p0_08, nrf_peripherals.UARTE0);

    let mut rx_buffer = [0; 1];
    let mut postcard_buffer = Vec::<_, { common::POSTCARD_BUFFER_SIZE }>::new();

    defmt::info!("ready to receive firmware image");
    loop {
        serial_port.read(&mut rx_buffer).unwrap();
        let byte = rx_buffer[0];
        postcard_buffer.push(byte).unwrap();

        if byte == common::COBS_DELIMITER {
            let request: Host2TargetMessage =
                postcard::from_bytes_cobs(&mut postcard_buffer).unwrap();

            let response = handle_request(request, &core_peripherals.SCB, &mut leds);
            let response_bytes =
                postcard::to_vec_cobs::<_, { common::POSTCARD_BUFFER_SIZE }>(&response).unwrap();

            serial_port.write(&response_bytes).unwrap();
            postcard_buffer.clear();
        }
    }
}

fn handle_request(request: Host2TargetMessage, scb: &SCB, leds: &mut Leds) -> Target2HostMessage {
    match request {
        Host2TargetMessage::Write {
            start_address,
            data,
        } => {
            let end_address = start_address + data.len() as u32;
            if is_valid_address_range(start_address..end_address) {
                let src = data.as_ptr();
                let dst = start_address as usize as *mut u8;
                let len = data.len();

                unsafe { core::ptr::copy_nonoverlapping(src, dst, len) }

                Target2HostMessage::WriteOk
            } else {
                defmt::error!(
                    "address range `{}..{}` is invalid",
                    start_address,
                    end_address
                );

                Target2HostMessage::InvalidAddress
            }
        }

        Host2TargetMessage::Execute => {
            defmt::info!("booting into new firmware...");

            // point VTOR to new vector table
            unsafe { scb.vtor.write(RAM_PROGRAM_START_ADDRESS) }

            leds.off();

            // flush defmt messages
            cortex_m::asm::delay(1_000_000);

            unsafe { cortex_m::asm::bootload(RAM_PROGRAM_START_ADDRESS as *const u32) }
        }
    }
}

fn is_valid_address_range(range: Range<u32>) -> bool {
    VALID_RAM_PROGRAM_ADDRESS.contains(&range.start)
        && VALID_RAM_PROGRAM_ADDRESS.contains(&range.end)
}

fn initialize_serial_port(
    p0_06: P0_06<Disconnected>,
    p0_08: P0_08<Disconnected>,
    uarte: UARTE0,
) -> Uarte<nrf52840_hal::pac::UARTE0> {
    let cdc_pins = uarte::Pins {
        txd: p0_06.into_push_pull_output(gpio::Level::High).degrade(),
        rxd: p0_08.into_floating_input().degrade(),
        cts: None,
        rts: None,
    };

    Uarte::new(
        uarte,
        cdc_pins,
        uarte::Parity::EXCLUDED,
        uarte::Baudrate::BAUD115200,
    )
}

struct Leds {
    p0_14: P0_14<Output<PushPull>>,
    p0_15: P0_15<Output<PushPull>>,
    p0_16: P0_16<Output<PushPull>>,
}

impl Leds {
    fn on(
        p0_14: P0_14<Disconnected>,
        p0_15: P0_15<Disconnected>,
        p0_16: P0_16<Disconnected>,
    ) -> Self {
        Self {
            p0_14: p0_14.into_push_pull_output(Level::Low),
            p0_15: p0_15.into_push_pull_output(Level::Low),
            p0_16: p0_16.into_push_pull_output(Level::Low),
        }
    }

    fn off(&mut self) {
        self.p0_14.set_high().unwrap();
        self.p0_15.set_high().unwrap();
        self.p0_16.set_high().unwrap();
    }
}
