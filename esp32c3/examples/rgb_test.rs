//! //! RGB LED Demo
//!
//! This example drives an SK68XX RGB LED that is connected to the GPIO8 pin.
//! A RGB LED is connected to that pin on the ESP32-C3-DevKitM-1 and
//! ESP32-C3-DevKitC-02 boards.
//!
//! The demo will leverage the [`smart_leds`](https://crates.io/crates/smart-leds)
//! crate functionality to circle through the HSV hue color space (with
//! saturation and value both at 255). Additionally, we apply a gamma correction
//! and limit the brightness to 10 (out of 255).
#![no_std]
#![no_main]

use esp32c3_hal::{clock::ClockControl, peripherals, prelude::*, rmt::Rmt, Delay, IO, Rtc};
//use esp_backtrace as _;
use esp_hal_smartled::{smartLedAdapter, SmartLedsAdapter};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};
use smart_leds::{
    brightness, gamma,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite,
};

#[entry]
fn main() -> ! {
    //generate rtt symbol for panic impl
    rtt_init_print!();
    let peripherals = peripherals::Peripherals::take();
    let mut system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let rtc = Rtc::new(peripherals.RTC_CNTL);

    // Configure RMT peripheral globally
    let rmt = Rmt::new(
        peripherals.RMT,
        80u32.MHz(),
        &mut system.peripheral_clock_control,
        &clocks,
    )
    .unwrap();

    // We use one of the RMT channels to instantiate a `SmartLedsAdapter` which can
    // be used directly with all `smart_led` implementations
    let mut led = <smartLedAdapter!(0, 1)>::new(rmt.channel0, io.pins.gpio2);

    // Initialize the Delay peripheral, and use it to toggle the LED state in a
    // loop.
    let mut delay = Delay::new(&clocks);

    let mut delay_val: u32 = 400;

    let mut led_on: bool = false;

    // for debug purposes use minutes vs hours
    let dawn_start_sec    = 3*60 ;//*60;
    let noon_start_sec    = 9*60 ;//*60;
    let evening_start_sec = 15*60;//*60;
    let night_start_sec   = 21*60;//*60;


    let mut color = Hsv {
        hue: 0,
        sat: 255,
        val: 255,
    };

    // COLORS: 
    // Dawn,    Aureolin,    #F8F32B -> H:  59/360, S: 82.7%, V: 97.3% 
    // Noon,    Ice blue,    #9CFFFA -> H: 177/360, S: 38.8%, V: 100%
    // Evening, Indigo dye,  #053C5E -> H: 203/360, S: 94.7%, V: 36.9%
    // Night,   Dark purple, #31081F -> H: 326/360, S: 83.7%, V: 19.2%

    let dawn = Hsv {
        hue: 42,
        sat: 211,
        val: 248,
    };

    let noon = Hsv {
        hue: 125,
        sat: 99,
        val: 255,
    };

    let evening = Hsv {
        hue: 144,
        sat: 241,
        val: 224, // should be 94
    };

    let night = Hsv {
        hue: 231,
        sat: 213,
        val: 240, // should be 48
    };

    let led_off = Hsv {
        hue: 0,
        sat: 0,
        val: 0,
    };

    let mut data;

    loop {
        // Iterate over the rainbow!
        //for hour in 0..=23 {
            let time_sec = rtc.get_time_ms() / 1000;
            if  time_sec >= dawn_start_sec && 
                time_sec <  noon_start_sec {
                color = dawn;
            } else if   time_sec >= noon_start_sec && 
                        time_sec < evening_start_sec {
                color = noon;
            } else if   time_sec >= evening_start_sec && 
                        time_sec < night_start_sec {
                color = evening;
            } else {
                color = night;
            }

            if !led_on {
                color = led_off;
            }

            rprintln!("time (seconds):{}", time_sec);
            // Convert from the HSV color space (where we can easily transition from one
            // color to the other) to the RGB color space that we can then send to the LED
            data = [hsv2rgb(color)];
            // When sending to the LED, we do a gamma correction first (see smart_leds
            // documentation for details) and then limit the brightness to 10 out of 255 so
            // that the output it's not too bright.
            led.write(brightness(gamma(data.iter().cloned()), 10))
                .unwrap();
            //rprintln!("rtc time {}", rtc.get_time_ms()/1024);
            led_on = !led_on;
            delay.delay_ms(delay_val);
        //}
    }
}
