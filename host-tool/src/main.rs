use std::time::Duration;

use common::{Host2TargetMessage, Target2HostMessage};

const TIMEOUT: Duration = Duration::from_secs(5);
const BAUD_RATE: u32 = 115_200;

fn main() -> color_eyre::Result<()> {
    // TODO use VID PID to find correct port
    const PORT: &str = "/dev/ttyACM0";
    let mut port = serialport::new(PORT, BAUD_RATE).open()?;
    port.set_timeout(TIMEOUT)?;

    let request = Host2TargetMessage::Ping;
    let request_bytes = postcard::to_stdvec_cobs(&request)?;
    dbg!(&request_bytes);

    port.write_all(&request_bytes)?;

    let mut response_buffer = [0; 256];
    let size = port.read(&mut response_buffer)?;

    let response: Target2HostMessage = postcard::from_bytes_cobs(&mut response_buffer)?;
    dbg!(&response_buffer[..size]);
    dbg!(response);

    Ok(())
}
