//! host side application
//!
//! Run on target `cd esp32c3`
//!
//! cargo embed --example cmd_crc_cobs_lib --release
//!
//! Run on host `cd host`
//!
//! cargo run
//!

// Rust dependencies
use std::{io::{Read, ErrorKind}, mem::size_of, time::Duration};

// Libraries
use corncobs::{max_encoded_len, ZERO};
use serial2::SerialPort;
use chrono::prelude::*;

// Application dependencies
use host::open;
use shared::{deserialize_crc_cobs, serialize_crc_cobs, Command, Message, Response, Faults, date_time::UtcDateTime}; // local library

const CMD_TIMEOUT_SECS : Duration = Duration::from_secs(2); 

const IN_SIZE: usize = max_encoded_len(size_of::<Response>() + size_of::<u32>());
const OUT_SIZE: usize = max_encoded_len(size_of::<Command>() + size_of::<u32>());

type InBuf = [u8; IN_SIZE];
type OutBuf = [u8; OUT_SIZE];

fn main() -> Result<(), std::io::Error> {

    // set to 1 to enable bit flip detection test
    let bit_flip_test : bool = false;

    println!("\n\nRTIC2 - Reliable Serial Communication: Host Application\n");

    // get current time
    let utc : DateTime<Utc> = Utc::now();
    let mut port = open()?;

    port.set_read_timeout(CMD_TIMEOUT_SECS)?;
    println!("Command timeout set to {:?} second(s).\n", port.get_read_timeout().unwrap().as_secs());

    let mut out_buf = [0u8; OUT_SIZE];
    let mut in_buf = [0u8; IN_SIZE];
    // cast chrono object into serdes friendly object
    let udt : UtcDateTime = utc.into();   

    // set time to current UTC time
    let cmd = Command::Set(0x1, Message::A(udt), 0b001);
    println!("--> Request: {:?}\n", cmd);
    let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf, bit_flip_test)?;
    println!("<-- Response: {:?}\n", response);
        
    // turn off blinker right now
    let cmd = Command::Set(0x2, Message::B(0), 0b001);
    println!("--> Request: {:?}\n", cmd);
    let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf, bit_flip_test)?;
    println!("<-- Response: {:?}\n", response);    

    // turn on blinker right now for set duration and frequency
    let cmd = Command::Set(0x3, Message::C(20, 10), 0b001);
    println!("--> Request: {:?}\n", cmd);
    let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf, bit_flip_test)?;
    println!("<-- Response: {:?}\n", response);
    
    let udt : UtcDateTime = UtcDateTime { year: 2023, month: 11, day: 23, hour: 18, minute: 35, second: 1, nanoseconds: 48 };    
    
    // schedule blinker for certain time for a set duration and frequency
    let cmd = Command::Set(0x4, Message::D(udt, 100, 32768), 0b001);
    println!("--> Request: {:?}\n", cmd);
    let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf, bit_flip_test)?;
    println!("<-- Response: {:?}\n", response);

    // toggle RGB LED
    let cmd = Command::Set(0x5, Message::B(0), 0b001);
    println!("--> Request: {:?}\n", cmd);
    let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf, bit_flip_test)?;
    println!("<-- Response: {:?}\n", response);

    // currently no use for get
    let cmd = Command::Get(0x12, 12, 0b001);
    println!("--> Request: {:?}\n", cmd);
    let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf, bit_flip_test)?;
    println!("<-- Response: {:?}\n", response);

    Ok(())
}

fn get_response(in_buf: &mut InBuf) -> Result<Response, ()> {
    
    // Get response and check for errors
    let rsp = deserialize_crc_cobs(in_buf);
    match rsp {
        Ok(r) => {
            return Ok(r);
        },
        Err(e) => {
            
            match e {
                Faults::BitFlipData => {
                    println!("[Error] Detected bit flip in Data or CRC!\n");
                },
            }; 
            return Ok(Response::NotOK);
        },
    };
}

fn request(
    cmd: &Command,
    port: &mut SerialPort,
    out_buf: &mut OutBuf,
    in_buf: &mut InBuf,
    bit_flip_test: bool,
) -> Result<Response, std::io::Error> {
    
    let to_write = serialize_crc_cobs(cmd, out_buf, bit_flip_test);
    let mut tx_complete : bool = false;

    while tx_complete == false {

        port.write_all(to_write)?;

        println!("Request written... Awaiting response.\n");

        let mut index: usize = 0;
        
        loop {
            
            let slice = &mut in_buf[index..index + 1];
            
            if index < IN_SIZE {
                index += 1;
            }
            
            match port.read_exact(slice) {
                Ok(_) => {
                    // do nothing
                },
                // check for timeout and re-send packet if detected
                Err(e) => {
                    match e.kind() {
                        ErrorKind::TimedOut => {
                            println!("[Error] - Request time-out expired!\n");
                            break;
                        },
                        _ => {
                            println!("[Error] - There was a problem reading a byte from the buffer: {:?}\n", e);
                            return Err(e);
                        },
                    };
                },
            };

            if slice[0] == ZERO {
                println!("Response received!\n");
                tx_complete = true;
                break;
            }
        }
    }

    // Get response and check for errors
    Ok(get_response(in_buf).unwrap())
   
}
