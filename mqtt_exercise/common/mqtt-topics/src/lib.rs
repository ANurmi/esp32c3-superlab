#![no_std]
// Formatting related imports
extern crate heapless;
use core::fmt::Write;
use heapless::String;

#[derive(Debug)]
pub enum Esp {
    EspTarget1 = 1,
    EspTarget2 = 2,
    // Add targets according to the number of connected ESPs
}

pub fn cmd_topic_fragment(uuid: &str, esp: Esp) -> String<256> {
    let mut buffer: String<256> = String::new();
    write!(&mut buffer, "{}/{:?}/command/", uuid, esp).expect("write! failed!");
    buffer
}

pub fn temperature_data_topic(uuid: &str, esp: Esp) -> String<256> {
    let mut buffer: String<256> = String::new();
    write!(&mut buffer, "{}/sensor_data/temperature{:?}", uuid, esp).expect("write! failed!");
    buffer
}

pub fn humidity_data_topic(uuid: &str, esp: Esp) -> String<256> {
    let mut buffer: String<256> = String::new();
    write!(&mut buffer, "{}/sensor_data/humidity{:?}", uuid, esp).expect("write! failed!");
    buffer
}

pub fn hello_topic(uuid: &str, esp: Esp) -> String<256> {
    let mut buffer: String<256> = String::new();
    write!(&mut buffer, "{}/hello_esp{:?}", uuid, esp).expect("write! failed!");
    buffer
}
