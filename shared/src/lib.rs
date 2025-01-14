#![cfg_attr(not(test), no_std)]

pub mod date_time;
pub mod shift_register;

use date_time::UtcDateTime;
use serde_derive::{Deserialize, Serialize};

// we could use new-type pattern here but let's keep it simple
pub type Id = u32;
pub type DevId = u32;
pub type Parameter = u32;

#[derive(Debug, Serialize, Deserialize)]
#[repr(C)]
pub enum Command {
    Set(Id, Message, DevId),
    Get(Id, Parameter, DevId),
}

#[derive(Debug, Serialize, Deserialize)]
#[repr(C)]
pub enum Message {
    A(UtcDateTime),
    B(u32),
    C(u32, u32), // we might consider "f16" but not sure it plays well with `ssmarshal`
    D(UtcDateTime, u32, u32),
}

#[derive(Debug, Serialize, Deserialize)]
#[repr(C)]
pub enum Response {
    Data(Id, Parameter, u32, DevId),
    SetOk,
    ParseError,
    NotOK,
    Illegal,
}

#[derive(Debug, Serialize, Deserialize)]
#[repr(C)]
pub enum Faults {
    BitFlipData,
}

pub const CKSUM: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_CKSUM);

/// Serialize T into cobs encoded out_buf with crc
/// panics on all errors
/// TODO: reasonable error handling
pub fn serialize_crc_cobs<'a, T: serde::Serialize, const N: usize>(
    t: &T,
    out_buf: &'a mut [u8; N],
    test_mode : bool,
) -> &'a [u8] {
    let n_ser = ssmarshal::serialize(out_buf, t).unwrap();
    let mut crc = CKSUM.checksum(&out_buf[0..n_ser]);
    
    if test_mode == true {
        crc = crc + 1;
    }
    
    let n_crc = ssmarshal::serialize(&mut out_buf[n_ser..], &crc).unwrap();
    let buf_copy = *out_buf; // implies memcpy, could we do better?
    let n = corncobs::encode_buf(&buf_copy[0..n_ser + n_crc], out_buf);
    &out_buf[0..n]
}

/// deserialize T from cobs in_buf with crc check
/// panics on all errors
pub fn deserialize_crc_cobs<T>(in_buf: &mut [u8]) -> Result<T, Faults>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let n = corncobs::decode_in_place(in_buf).unwrap();
    let (t, resp_used) = ssmarshal::deserialize::<T>(&in_buf[0..n]).unwrap();
    let crc_buf = &in_buf[resp_used..];
    let (crc, _crc_used) = ssmarshal::deserialize::<u32>(crc_buf).unwrap();
    let pkg_crc = CKSUM.checksum(&in_buf[0..resp_used]);

    // check for bitflip within payload/CRC
    if crc != pkg_crc {
        return Err(Faults::BitFlipData);
    }

    Ok(t)
}
