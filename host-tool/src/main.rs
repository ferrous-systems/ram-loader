use std::time::Duration;

use serde_derive::Serialize;

#[derive(Serialize)]
enum Host2TargetMessage {
    Ping,
}

enum Target2HostMessage {
    Pong,
}

const TIMEOUT: Duration = Duration::from_secs(5);
const BAUD_RATE: u32 = 115_200;

fn main() -> color_eyre::Result<()> {
    // TODO use VID PID to find correct port
    const PORT: &str = "/dev/ttyACM0";
    let mut port = serialport::new(PORT, BAUD_RATE).open()?;
    port.set_timeout(TIMEOUT);

    // let request = Host2TargetMessage::Ping;
    // let request_bytes = postcard::to_stdvec_cobs(&request)?;
    let request_bytes = [b'H'];

    port.write_all(&request_bytes)?;

    let mut response_buffer = [0];
    port.read(&mut response_buffer)?;

    dbg!(response_buffer);

    assert_eq!(request_bytes, response_buffer);

    Ok(())
}
