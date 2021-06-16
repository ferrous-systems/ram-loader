#![cfg_attr(not(test), no_std)]

use defmt::Format;
use serde_derive::{Deserialize, Serialize};

pub const COBS_DELIMITER: u8 = 0;
pub const POSTCARD_BUFFER_SIZE: usize = 256;
// pub const POSTCARD_PAYLOAD_SIZE: usize = 240;
// TODO make this larger
pub const POSTCARD_PAYLOAD_SIZE: usize = 3;

#[derive(Debug, Format, Deserialize, Serialize)]
pub enum Host2TargetMessage<'a> {
    Ping,
    Write { start_address: u32, data: &'a [u8] },
}

#[derive(Debug, Format, Deserialize, PartialEq, Serialize)]
pub enum Target2HostMessage {
    InvalidAddress,
    Pong,
    WriteOk,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_fits_in_buffer_size_bytes() {
        let dummy_data = vec![0; POSTCARD_PAYLOAD_SIZE];
        let message = Host2TargetMessage::Write {
            start_address: 0,
            data: &dummy_data,
        };
        let res = postcard::to_vec_cobs::<_, POSTCARD_BUFFER_SIZE>(&message);
        assert!(res.is_ok());
    }
}
