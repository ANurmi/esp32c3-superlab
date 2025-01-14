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

    // COMMANDS SEQUENCE
    // Executed once per host program invocation
    // use true/false to enable or disable the used set of command

    if true { // set time to current UTC time
        let cmd = dt_set_cmd();
        println!("--> Request: {:?}\n", cmd);
        let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf, bit_flip_test)?;
        println!("<-- Response: {:?}\n", response);
    }

    if false { // turn off blinker right now
        let cmd = blink_off_cmd();
        println!("--> Request: {:?}\n", cmd);
        let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf, bit_flip_test)?;
        println!("<-- Response: {:?}\n", response);  
    }    
  
    if false { // turn on blinker right now for set duration and frequency
        let cmd = blink_on_cmd(10, 3);
        println!("--> Request: {:?}\n", cmd);
        let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf, bit_flip_test)?;
        println!("<-- Response: {:?}\n", response);
    }

    // Test time for absolute scheduling
    let mut udt : UtcDateTime = Utc::now().into();
    udt.minute += 1;


    if false {
        // schedule blinker for absolute time for a set duration and frequency
        // note that this will return an illegal response if attempted before the time is set
        let cmd = blink_sched_abs_cmd(&udt, 10, 6);
        println!("--> Request: {:?}\n", cmd);
        let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf, bit_flip_test)?;
        println!("<-- Response: {:?}\n", response);
    }

    if true { // schedule blinker for a time with relative offset to current time
        let cmd = blink_sched_rel_cmd(5, 10, 6);
        println!("--> Request: {:?}\n", cmd);
        let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf, bit_flip_test)?;
        println!("<-- Response: {:?}\n", response);
    }

    if true { // set state of rbg led, true->on : false->off
        let cmd = set_rgb_on_cmd(true);
        println!("--> Request: {:?}\n", cmd);
        let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf, bit_flip_test)?;
        println!("<-- Response: {:?}\n", response);
    }

    //if true {    
    //    // currently no use for get
    //    let cmd = Command::Get(0x12, 12, 0b001);
    //    println!("--> Request: {:?}\n", cmd);
    //    let response = request(&cmd, &mut port, &mut out_buf, &mut in_buf, bit_flip_test)?;
    //    println!("<-- Response: {:?}\n", response);
    //}

    Ok(())
}

fn dt_set_cmd() -> Command {
    let utc : DateTime<Utc> = Utc::now();
    let udt : UtcDateTime   = utc.into();
    let cmd : Command       = Command::Set(0x1, Message::A(udt), 0b001);
    cmd
}

fn blink_off_cmd() -> Command {
    let cmd = Command::Set(0x2, Message::B(0), 0b001);
    cmd
}
fn blink_on_cmd(blk_dur: u32, blk_freq: u32)-> Command {
    let cmd = Command::Set(0x3, Message::C(blk_dur,blk_freq), 0b001);
    cmd
}
fn blink_sched_abs_cmd(utc_dt: &UtcDateTime, blk_dur: u32, blk_freq: u32) -> Command {

    let tmp_utc_dt = UtcDateTime {
                                    year:           utc_dt.year, 
                                    month:          utc_dt.month, 
                                    day:            utc_dt.day, 
                                    hour:           utc_dt.hour, 
                                    minute:         utc_dt.minute, 
                                    second:         utc_dt.second, 
                                    nanoseconds:    utc_dt.nanoseconds
                                };

    let cmd = Command::Set(0x4, Message::D(tmp_utc_dt, blk_dur, blk_freq), 0b001);
    cmd
}
fn blink_sched_rel_cmd(offset_secs: i64, blk_dur: u32, blk_freq: u32) -> Command {
    let udt         : UtcDateTime   = Utc::now().into();
    let dt          : DateTime<Utc> = Utc.with_ymd_and_hms( udt.year, 
                                                            udt.month, 
                                                            udt.day, 
                                                            udt.hour,
                                                        udt.minute, 
                                                        udt.second
                                                    ).unwrap();
    let epoch_millis: i64           = dt.timestamp_millis();
    let offset      : i64           = epoch_millis + offset_secs*1000;
    let udt_new     : UtcDateTime   = Utc.timestamp_millis_opt(offset).unwrap().into();
    let cmd         : Command       = Command::Set(0x4, Message::D(udt_new, blk_dur, blk_freq), 0b001);
    cmd
}
fn set_rgb_on_cmd(state: bool) -> Command {
    let led_state: u32        = if state {1} else {0};
    let cmd      : Command    = Command::Set(0x5, Message::B(led_state), 0b001);
    cmd
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
