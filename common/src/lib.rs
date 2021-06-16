#![no_std]

use defmt::Format;
use serde_derive::{Deserialize, Serialize};

pub const COBS_DELIMITER: u8 = 0;

#[derive(Debug, Format, Deserialize, Serialize)]
pub enum Host2TargetMessage {
    Ping,
}

#[derive(Debug, Format, Deserialize, Serialize)]
pub enum Target2HostMessage {
    Pong,
}
