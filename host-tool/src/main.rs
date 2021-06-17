use std::{
    env, fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    time::Duration,
};

use color_eyre::eyre::{ensure, eyre};
use common::{Host2TargetMessage, Target2HostMessage};
use object::{
    elf::{self, FileHeader32},
    read::elf::{FileHeader, ProgramHeader},
    Endianness,
};
use serialport::SerialPort;
use std::io::Write;

const TIMEOUT: Duration = Duration::from_secs(5);
const BAUD_RATE: u32 = 115_200;

fn main() -> color_eyre::Result<()> {
    // TODO this should reject 2+ arguments
    let file_path = PathBuf::from(
        env::args()
            .nth(1)
            .ok_or_else(|| eyre!("expected one argument"))?,
    );

    println!("Sending ELF file at {:?} to target", &file_path);

    let segments = extract_loadable_segments(file_path)?;

    let mut conn = TargetConn::open()?;
    for segment in &segments {
        let mut start_address = segment.start_address;
        for chunk in segment.data.chunks(common::POSTCARD_PAYLOAD_SIZE) {
            let message = Host2TargetMessage::Write {
                start_address,
                data: chunk,
            };
            start_address += chunk.len() as u32;

            let response = conn.request_response(message)?;

            ensure!(
                response == Target2HostMessage::WriteOk,
                "write operation failed"
            );

            // rudimentary progress indicator
            print!(".");
            std::io::stdout().flush()?;
        }
    }

    conn.send(Host2TargetMessage::Execute)?;
    println!("\nprogram loaded");

    Ok(())
}

/// Loadable segment
#[derive(Debug)]
struct Segment {
    start_address: u32,
    data: Vec<u8>,
}

/// Extracts loadable segments from the ELF
///
/// Most of these will map to linker sections like `.text` and `.rodata`
/// For `.data`, a section whose *Virtual* memory address is different than its *Load* / *Physical*,
///  this returns the *physical* memory address
// no high level API for this in the `object` crate. the `probe-rs` crate does something similar
fn extract_loadable_segments(file_path: PathBuf) -> Result<Vec<Segment>, color_eyre::Report> {
    let bytes = fs::read(file_path)?;
    let file_header = FileHeader32::<Endianness>::parse(&*bytes)?;
    let endianness = file_header.endian()?;

    let mut segments = vec![];
    for program_header in file_header.program_headers(endianness, &*bytes)? {
        let p_type = program_header.p_type(endianness);

        if p_type == elf::PT_LOAD {
            let p_paddr = program_header.p_paddr(endianness);
            let data = program_header
                .data(endianness, &*bytes)
                .map_err(|__| eyre!("cannot retreive program header data"))?;

            segments.push(Segment {
                start_address: p_paddr,
                data: data.to_vec(),
            });
        }
    }

    Ok(segments)
}

/// A connection to a nRF52840 Development Kit running the `ramloader` firmware
struct TargetConn {
    reader: BufReader<Box<dyn SerialPort>>,
    writer: Box<dyn SerialPort>,
}

impl TargetConn {
    /// Opens a connection to the `ramloader` "target"
    fn open() -> color_eyre::Result<Self> {
        const VID: u16 = 0x1366;
        const PID: u16 = 0x1015;

        let mut port_info = None;
        for port in serialport::available_ports()? {
            match &port.port_type {
                serialport::SerialPortType::UsbPort(info) => {
                    if info.vid == VID && info.pid == PID {
                        port_info = Some(port);
                    }
                }
                _ => continue,
            }
        }

        let port_info =
            port_info.ok_or_else(|| eyre!("serial port `{:04x}:{:04x}` not found", VID, PID))?;
        let mut port = serialport::new(port_info.port_name, BAUD_RATE).open()?;
        port.set_timeout(TIMEOUT)?;

        let writer = port.try_clone()?;
        let reader = BufReader::new(port);

        Ok(Self { writer, reader })
    }

    /// Sends request and waits for a response
    fn request_response(
        &mut self,
        request: Host2TargetMessage,
    ) -> color_eyre::Result<Target2HostMessage> {
        self.send(request)?;

        let mut response_buffer = vec![];
        self.reader
            .read_until(common::COBS_DELIMITER, &mut response_buffer)?;

        let response: Target2HostMessage = postcard::from_bytes_cobs(&mut response_buffer)?;
        Ok(response)
    }

    /// Sends a request but does not wait for a response
    fn send(&mut self, request: Host2TargetMessage) -> color_eyre::Result<()> {
        let request_bytes = postcard::to_vec_cobs::<_, { common::POSTCARD_BUFFER_SIZE }>(&request)?;

        self.writer.write_all(&request_bytes)?;
        Ok(())
    }
}
