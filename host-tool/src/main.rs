use std::{
    env, fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    time::Duration,
};

use color_eyre::eyre::eyre;
use common::{Host2TargetMessage, Target2HostMessage};
use object::{
    elf::{FileHeader32, PT_LOAD},
    read::elf::{FileHeader, ProgramHeader},
    Endianness,
};
use serialport::SerialPort;

const TIMEOUT: Duration = Duration::from_secs(5);
const BAUD_RATE: u32 = 115_200;

fn main() -> color_eyre::Result<()> {
    // TODO doesn't reject 2+ arguments
    let file_path = PathBuf::from(
        env::args()
            .nth(1)
            .ok_or_else(|| eyre!("expected one argument"))?,
    );

    dbg!(&file_path);

    let segments = extract_loadable_segments(file_path)?;

    let mut conn = TargetConn::new()?;
    for segment in &segments {
        let mut start_address = segment.start_address;
        for chunk in segment.data.chunks(common::POSTCARD_PAYLOAD_SIZE) {
            let message = Host2TargetMessage::Write {
                start_address,
                data: chunk,
            };
            start_address += chunk.len() as u32;

            let response = conn.request_response(message)?;

            // TODO emit error message (specially for invalid address)
            assert_eq!(response, Target2HostMessage::WriteOk);
        }
    }

    conn.send(Host2TargetMessage::Execute)?;
    println!("program loaded");

    Ok(())
}

/// Loadable segment
#[derive(Debug)]
struct Segment {
    start_address: u32,
    data: Vec<u8>,
}

fn extract_loadable_segments(file_path: PathBuf) -> Result<Vec<Segment>, color_eyre::Report> {
    let bytes = fs::read(file_path)?;
    let file_header = FileHeader32::<Endianness>::parse(&*bytes)?;
    let endianness = file_header.endian()?;

    let mut segments = vec![];
    for program_header in file_header.program_headers(endianness, &*bytes)? {
        let p_type = program_header.p_type(endianness);

        if p_type == PT_LOAD {
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

struct TargetConn {
    reader: BufReader<Box<dyn SerialPort>>,
    writer: Box<dyn SerialPort>,
}

impl TargetConn {
    fn new() -> color_eyre::Result<Self> {
        // TODO use VID PID to find correct port
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
        let mut port = serialport::new(dbg!(port_info.port_name), BAUD_RATE).open()?;
        port.set_timeout(TIMEOUT)?;

        let writer = port.try_clone()?;
        let reader = BufReader::new(port);

        Ok(Self { writer, reader })
    }

    fn request_response(
        &mut self,
        request: Host2TargetMessage,
    ) -> color_eyre::Result<Target2HostMessage> {
        self.send(request)?;

        let mut response_buffer = vec![];
        self.reader
            .read_until(common::COBS_DELIMITER, &mut response_buffer)?;

        dbg!(&response_buffer);

        let response: Target2HostMessage = postcard::from_bytes_cobs(&mut response_buffer)?;
        Ok(response)
    }

    fn send(&mut self, request: Host2TargetMessage) -> color_eyre::Result<()> {
        let request_bytes = postcard::to_vec_cobs::<_, { common::POSTCARD_BUFFER_SIZE }>(&request)?;
        dbg!(request_bytes.len());

        self.writer.write_all(&request_bytes)?;
        Ok(())
    }
}
