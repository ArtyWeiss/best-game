#![allow(unused)]

use std::fmt::Display;

use core::fmt;
use std::str::from_utf8;
use std::time::{SystemTime, UNIX_EPOCH};

// OTHER OPTIONS ??
//
// colored(184, 187, 38, "INFO")
// colored(250, 189, 47, "WARNING")

#[inline(always)]
pub fn trace<S: Display>(text: S) {
    let time = format_system_time(&SystemTime::now(), Precision::Millis).unwrap();
    let time_text = colored(151, 144, 136, &time);
    let tag_text = colored(131, 165, 152, "TRACE");
    println!("[{time_text}] [{tag_text}] {text}");
}

#[inline(always)]
pub fn error<S: Display>(text: S) {
    let time = format_system_time(&SystemTime::now(), Precision::Millis).unwrap();
    let time_text = colored(151, 144, 136, &time);
    let tag_text = colored(251, 73, 52, "ERROR");
    println!("[{time_text}] [{tag_text}] {text}");
}

// PRIVATE FUNCTIONS ==========================================================================
#[inline(always)]
fn colored<S: Display>(r: i32, g: i32, b: i32, text: S) -> String {
    format!("\x1B[38;2;{};{};{}m{}\x1B[0m", r, g, b, text)
}

#[derive(Clone, PartialEq, Eq)]
enum Precision {
    Smart,
    Seconds,
    Millis,
    Micros,
    Nanos,
}

/// RFC 3339
fn format_system_time(time: &SystemTime, precision: Precision) -> Result<String, fmt::Error> {
    use self::Precision::*;

    let dur = time
        .duration_since(UNIX_EPOCH)
        .expect("all times should be after the epoch");
    let secs_since_epoch = dur.as_secs();
    let nanos = dur.subsec_nanos();

    if secs_since_epoch >= 253_402_300_800 {
        // year 9999
        return Err(fmt::Error);
    }

    /* 2000-03-01 (mod 400 year, immediately after feb29 */
    const LEAPOCH: i64 = 11017;
    const DAYS_PER_400Y: i64 = 365 * 400 + 97;
    const DAYS_PER_100Y: i64 = 365 * 100 + 24;
    const DAYS_PER_4Y: i64 = 365 * 4 + 1;

    let days = (secs_since_epoch / 86400) as i64 - LEAPOCH;
    let secs_of_day = secs_since_epoch % 86400;

    let mut qc_cycles = days / DAYS_PER_400Y;
    let mut remdays = days % DAYS_PER_400Y;

    if remdays < 0 {
        remdays += DAYS_PER_400Y;
        qc_cycles -= 1;
    }

    let mut c_cycles = remdays / DAYS_PER_100Y;
    if c_cycles == 4 {
        c_cycles -= 1;
    }
    remdays -= c_cycles * DAYS_PER_100Y;

    let mut q_cycles = remdays / DAYS_PER_4Y;
    if q_cycles == 25 {
        q_cycles -= 1;
    }
    remdays -= q_cycles * DAYS_PER_4Y;

    let mut remyears = remdays / 365;
    if remyears == 4 {
        remyears -= 1;
    }
    remdays -= remyears * 365;

    let mut year = 2000 + remyears + 4 * q_cycles + 100 * c_cycles + 400 * qc_cycles;

    let months = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];
    let mut mon = 0;
    for mon_len in months.iter() {
        mon += 1;
        if remdays < *mon_len {
            break;
        }
        remdays -= *mon_len;
    }
    let mday = remdays + 1;
    let mon = if mon + 2 > 12 {
        year += 1;
        mon - 10
    } else {
        mon + 2
    };

    let mut buf: [u8; 30] = [
        // Too long to write as: b"0000-00-00T00:00:00.000000000Z"
        b'0', b'0', b'0', b'0', b'-', b'0', b'0', b'-', b'0', b'0', b'T', b'0', b'0', b':', b'0',
        b'0', b':', b'0', b'0', b'.', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'Z',
    ];
    buf[0] = b'0' + (year / 1000) as u8;
    buf[1] = b'0' + (year / 100 % 10) as u8;
    buf[2] = b'0' + (year / 10 % 10) as u8;
    buf[3] = b'0' + (year % 10) as u8;
    buf[5] = b'0' + (mon / 10) as u8;
    buf[6] = b'0' + (mon % 10) as u8;
    buf[8] = b'0' + (mday / 10) as u8;
    buf[9] = b'0' + (mday % 10) as u8;
    buf[11] = b'0' + (secs_of_day / 3600 / 10) as u8;
    buf[12] = b'0' + (secs_of_day / 3600 % 10) as u8;
    buf[14] = b'0' + (secs_of_day / 60 / 10 % 6) as u8;
    buf[15] = b'0' + (secs_of_day / 60 % 10) as u8;
    buf[17] = b'0' + (secs_of_day / 10 % 6) as u8;
    buf[18] = b'0' + (secs_of_day % 10) as u8;

    let offset = if precision == Seconds || nanos == 0 && precision == Smart {
        buf[19] = b'Z';
        19
    } else if precision == Millis {
        buf[20] = b'0' + (nanos / 100_000_000) as u8;
        buf[21] = b'0' + (nanos / 10_000_000 % 10) as u8;
        buf[22] = b'0' + (nanos / 1_000_000 % 10) as u8;
        buf[23] = b'Z';
        23
    } else if precision == Micros {
        buf[20] = b'0' + (nanos / 100_000_000) as u8;
        buf[21] = b'0' + (nanos / 10_000_000 % 10) as u8;
        buf[22] = b'0' + (nanos / 1_000_000 % 10) as u8;
        buf[23] = b'0' + (nanos / 100_000 % 10) as u8;
        buf[24] = b'0' + (nanos / 10_000 % 10) as u8;
        buf[25] = b'0' + (nanos / 1_000 % 10) as u8;
        buf[26] = b'Z';
        26
    } else {
        buf[20] = b'0' + (nanos / 100_000_000) as u8;
        buf[21] = b'0' + (nanos / 10_000_000 % 10) as u8;
        buf[22] = b'0' + (nanos / 1_000_000 % 10) as u8;
        buf[23] = b'0' + (nanos / 100_000 % 10) as u8;
        buf[24] = b'0' + (nanos / 10_000 % 10) as u8;
        buf[25] = b'0' + (nanos / 1_000 % 10) as u8;
        buf[26] = b'0' + (nanos / 100 % 10) as u8;
        buf[27] = b'0' + (nanos / 10 % 10) as u8;
        buf[28] = b'0' + (nanos / 1 % 10) as u8;
        // 29th is 'Z'
        29
    };

    // we know our chars are all ascii
    Ok(from_utf8(&buf[..=offset])
        .expect("Conversion to utf8 failed")
        .to_string())
}
