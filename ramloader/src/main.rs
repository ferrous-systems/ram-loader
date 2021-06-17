#![no_main]
#![no_std]

use core::ops::RangeInclusive;

use common::{Host2TargetMessage, Target2HostMessage};
use heapless::Vec;
use nrf52840_hal::{
    gpio::{self, p0, Level},
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
    // turn on some LEDs as a visual indicator
    port0_pins.p0_14.into_push_pull_output(Level::Low);
    port0_pins.p0_15.into_push_pull_output(Level::Low);
    port0_pins.p0_16.into_push_pull_output(Level::Low);

    let cdc_pins = uarte::Pins {
        txd: port0_pins
            .p0_06
            .into_push_pull_output(gpio::Level::High)
            .degrade(),
        rxd: port0_pins.p0_08.into_floating_input().degrade(),
        cts: None,
        rts: None,
    };

    let mut uarte = Uarte::new(
        nrf_peripherals.UARTE0,
        cdc_pins,
        uarte::Parity::EXCLUDED,
        uarte::Baudrate::BAUD115200,
    );

    let mut serial_rx_buffer = [0; 1];

    let mut cobs_buffer = Vec::<_, { common::POSTCARD_BUFFER_SIZE }>::new();
    defmt::info!("ready to receive firmware image");
    loop {
        uarte.read(&mut serial_rx_buffer).unwrap();
        let byte = serial_rx_buffer[0];
        cobs_buffer.push(byte).unwrap();

        if byte == common::COBS_DELIMITER {
            let host2target_message: Host2TargetMessage =
                postcard::from_bytes_cobs(&mut cobs_buffer).unwrap();

            let response = match host2target_message {
                Host2TargetMessage::Write {
                    start_address,
                    data,
                } => {
                    if VALID_RAM_PROGRAM_ADDRESS.contains(&start_address) {
                        if data.len() != 0 {
                            let end_address = start_address + data.len() as u32;
                            if VALID_RAM_PROGRAM_ADDRESS.contains(&end_address) {
                                let src = data.as_ptr();
                                let dst = start_address as usize as *mut u8;
                                let len = data.len();

                                unsafe {
                                    core::ptr::copy_nonoverlapping(src, dst, len);
                                }

                                Target2HostMessage::WriteOk
                            } else {
                                defmt::error!(
                                    "address range `{}..{}` is invalid",
                                    start_address,
                                    end_address
                                );
                                Target2HostMessage::InvalidAddress
                            }
                        } else {
                            Target2HostMessage::WriteOk
                        }
                    } else {
                        defmt::error!("address `{}` is invalid", start_address);
                        Target2HostMessage::InvalidAddress
                    }
                }

                Host2TargetMessage::Execute => {
                    defmt::info!("booting into new firmware...");

                    // write to VTOR
                    unsafe {
                        core_peripherals.SCB.vtor.write(RAM_PROGRAM_START_ADDRESS);
                    }

                    // flush defmt messages
                    cortex_m::asm::delay(1_000_000);

                    unsafe { cortex_m::asm::bootload(RAM_PROGRAM_START_ADDRESS as *const u32) }
                }
            };

            let response_bytes = postcard::to_vec_cobs::<_, 256>(&response).unwrap();

            uarte.write(&response_bytes).unwrap();
            cobs_buffer.clear();
        }
    }
}
