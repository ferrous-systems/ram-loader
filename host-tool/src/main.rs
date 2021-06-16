use std::{
    io::{BufRead, BufReader},
    time::Duration,
};

use common::{Host2TargetMessage, Target2HostMessage};
use serialport::SerialPort;

const TIMEOUT: Duration = Duration::from_secs(5);
const BAUD_RATE: u32 = 115_200;

fn main() -> color_eyre::Result<()> {
    let mut conn = TargetConn::new()?;

    let response = conn.request(Host2TargetMessage::Ping)?;
    dbg!(response);
    let response = conn.request(Host2TargetMessage::Ping)?;
    dbg!(response);

    Ok(())
}

struct TargetConn {
    reader: BufReader<Box<dyn SerialPort>>,
    writer: Box<dyn SerialPort>,
}

impl TargetConn {
    fn new() -> color_eyre::Result<Self> {
        // TODO use VID PID to find correct port
        const PORT: &str = "/dev/ttyACM0";
        let mut port = serialport::new(PORT, BAUD_RATE).open()?;
        port.set_timeout(TIMEOUT)?;

        let writer = port.try_clone()?;
        let reader = BufReader::new(port);

        Ok(Self { writer, reader })
    }

    fn request(&mut self, request: Host2TargetMessage) -> color_eyre::Result<Target2HostMessage> {
        const COBS_DELIMITER: u8 = 0;

        let request_bytes = postcard::to_stdvec_cobs(&request)?;
        dbg!(&request_bytes);

        self.writer.write_all(&request_bytes)?;

        let mut response_buffer = vec![];
        self.reader
            .read_until(COBS_DELIMITER, &mut response_buffer)?;

        dbg!(&response_buffer);

        let response: Target2HostMessage = postcard::from_bytes_cobs(&mut response_buffer)?;
        Ok(response)
    }
}
