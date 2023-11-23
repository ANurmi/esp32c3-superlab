//! serial prototype
//!
//! Run on target: `cd esp32c3`
//!
//! cargo embed --example serial_prototype 
//!
//! Run on host: `cd esp32c3`
//!
//! minicom -b 115200 -D /dev/ttyUSB0
//!
//! Module used to practice using COBS over serial
//!
//! This assumes we have usb<->serial adepter appearing as /dev/ACM1
//! - Target TX = GPIO0, connect to RX on adapter
//! - Target RX = GPIO1, connect to TX on adapter
//!
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![allow(unreachable_patterns)]
#![allow(dead_code)]

use panic_rtt_target as _;

// bring in panic handler
use panic_rtt_target as _;



#[rtic::app(device = esp32c3, dispatchers = [FROM_CPU_INTR0, FROM_CPU_INTR1])]
mod app {
    use esp32c3_hal::{
        rmt::Rmt,
        clock::ClockControl,
        peripherals::{Peripherals, TIMG0, TIMG1, UART0},
        prelude::*,
        timer::{Timer, Timer0, TimerGroup},
        gpio::{Gpio7, Output, PushPull},
        uart::{
            config::{Config, DataBits, Parity, StopBits},
            TxRxPins, UartRx, UartTx,
        },
        Rtc,
        Uart, IO,
    };

    pub struct BlinkLedConfig {
        blink_start_time : i64,
        blink_end_time : i64,
        blink_period_millis : u32,
        active : bool,
    }

    use rtic_sync::{channel::*, make_channel};
    use rtt_target::{rprintln, rtt_init_print};

    use smart_leds::{
        brightness,
        RGB,
        SmartLedsWrite,
    };

    use esp_hal_smartled::{smartLedAdapter, SmartLedsAdapter};


    use core::mem::size_of;

    use chrono::prelude::*;

    use chrono::{Utc};

    // shared libs
    use corncobs::{max_encoded_len, ZERO};
    use shared::{serialize_crc_cobs, Command, Message, Response, deserialize_crc_cobs}; // local library

    const IN_SIZE: usize = max_encoded_len(size_of::<Command>() + size_of::<u32>());
    const OUT_SIZE: usize = max_encoded_len(size_of::<Response>() + size_of::<u32>());

    type InBuf = [u8; IN_SIZE];
    type OutBuf = [u8; OUT_SIZE];

    const CAPACITY: usize = 100;

    #[shared]
    struct Shared {
      epoch_millis : i64,
      blink_led_config : BlinkLedConfig,
      tg0_timer0 : Timer<Timer0<TIMG0>>,
      blink_led: Gpio7<Output<PushPull>>,
      color_led_active : bool,
    }

    #[local]
    struct Local {
        color_led : SmartLedsAdapter<esp32c3_hal::rmt::Channel0<0>, 0, 25>,
        tg1_timer0 : Timer<Timer0<TIMG1>>,
        previous_rtc_timestamp : u64,
        rtc : Rtc<'static>,
        tx: UartTx<'static, UART0>,
        rx: UartRx<'static, UART0>,
        sender: Sender<'static, Response, CAPACITY>,
        rx_buff: InBuf,
        rx_idx: usize,
    }

    
    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        rtt_init_print!();
        rprintln!("uart_echo_split");
        let (sender, receiver) = make_channel!(Response, CAPACITY);

        let peripherals = Peripherals::take();
        let mut system = peripherals.SYSTEM.split();
        let clocks = ClockControl::max(system.clock_control).freeze();

        let timer_group0 = TimerGroup::new(
            peripherals.TIMG0,
            &clocks,
            &mut system.peripheral_clock_control,
        );
        let mut tg0_timer0 = timer_group0.timer0;

        let timer_group1 = TimerGroup::new(
          peripherals.TIMG1,
          &clocks,
          &mut system.peripheral_clock_control,
        );
        let mut tg1_timer0 = timer_group1.timer0;
        tg0_timer0.clear_interrupt();
        tg1_timer0.clear_interrupt();

        let config = Config {
            baudrate: 115200,
            data_bits: DataBits::DataBits8,
            parity: Parity::ParityNone,
            stop_bits: StopBits::STOP1,
        };

        let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
        let pins = TxRxPins::new_tx_rx(
            io.pins.gpio0.into_push_pull_output(),
            io.pins.gpio1.into_floating_input(),
        );

        let mut uart0 = Uart::new_with_config(
            peripherals.UART0,
            config,
            Some(pins),
            &clocks,
            &mut system.peripheral_clock_control,
        );

        // This is stupid!
        // TODO, use at commands with break character
        uart0.set_rx_fifo_full_threshold(1).unwrap();
        uart0.listen_rx_fifo_full();

        tg1_timer0.start(1u64.secs());
        tg0_timer0.start(1u64.secs());

        let (tx, rx) = uart0.split();

        let rx_buff : InBuf = [0; IN_SIZE];
        let rx_idx  : usize = 0;

        let rtc = Rtc::new(peripherals.RTC_CNTL);

        let dt = Utc.with_ymd_and_hms(2023, 1, 1,17, 0, 0).unwrap();
                  
        let epoch_millis = dt.timestamp_millis();

        let previous_rtc_timestamp = rtc.get_time_ms();

        let blink_led_config = BlinkLedConfig {
            blink_start_time: epoch_millis + 1000,
            blink_end_time: epoch_millis + 10000,
            blink_period_millis: 300,
            active : false,
        };


        uart_tx::spawn(receiver).unwrap();

        let mut blink_led = io.pins.gpio7.into_push_pull_output();

        blink_led.set_low().unwrap();

        tg1_timer0.listen();

        let rmt = Rmt::new(
            peripherals.RMT,
            80u32.MHz(),
            &mut system.peripheral_clock_control,
            &clocks,
        )
        .unwrap();

        let color_led: SmartLedsAdapter<esp32c3_hal::rmt::Channel0<0>, 0, 25> = <smartLedAdapter!(0, 1)>::new(rmt.channel0, io.pins.gpio2);
        let color_led_active = true;
        (
            Shared {
              epoch_millis,
              blink_led_config,
              tg0_timer0,
              blink_led,
              color_led_active,
            },
            Local {
              color_led,
              tg1_timer0,
              rtc,
              previous_rtc_timestamp,
              tx,
              rx,
              sender,
              rx_buff,
              rx_idx,
            },
        )
    }

    // notice this is not an async task
    #[idle(local = [ ])]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            //rprintln!("idle, do some background work if any ...");
            // not async wait
            //nb::block!(cx.local.timer0.wait()).unwrap();
        }
    }

    #[task(binds = UART0, priority=2, local = [ rx, sender, rx_buff, rx_idx], shared = [epoch_millis, blink_led_config, color_led_active])]
    fn uart0(mut cx: uart0::Context) {
        
        let rx = cx.local.rx;
        let sender = cx.local.sender;
        
        let rx_buff = cx.local.rx_buff;
        let rx_idx = cx.local.rx_idx;
        

        while let nb::Result::Ok(c) = rx.read() {

            rx_buff[*rx_idx] = c;

            // reset element idx counter when eof received
            if c == ZERO {
              
              *rx_idx = 0;

              let cmd = deserialize_crc_cobs(rx_buff).unwrap();

              match &cmd {

                Command::Set(id, msg, devid) => {

                  match &msg {
                    Message::A(udt) => {
                      rprintln!("Received Set({}, [year={}, month={}, day={}, hour={}, min={}, sec={}, nsec={}],{})", id, udt.year, udt.month, udt.day, udt.hour, udt.minute, udt.second, udt.nanoseconds, devid);
                      
                      let dt = Utc.with_ymd_and_hms(udt.year, udt.month, udt.day, udt.hour, udt.minute, udt.second).unwrap();
          
                      let new_epoch_millis = dt.timestamp_millis();

                      cx.shared.epoch_millis.lock(|epoch_millis| {
                        *epoch_millis = new_epoch_millis;
                      });
                    },
                    Message::B(int_val) => {
                      rprintln!("Received Set({},{},{})", id, int_val, devid);

                      if *id == 2 {
                        cx.shared.blink_led_config.lock(|config| {
                            // Set this to zero so we stop blinking
                            config.blink_end_time = 0;
                        });
                      } else if *id == 5 {
                        cx.shared.color_led_active.lock(|active| {
                            *active = *int_val != 0;
                        });
                      }

                    },
                    Message::C(duration_secs, freq_hz) => {
                      rprintln!("Received Set({},({} sec, {} Hz),{})", id, duration_secs, freq_hz, devid);

                        let mut time_stamp = 0;

                        //Avoid nested locks
                        cx.shared.epoch_millis.lock(|epoch_millis| {
                            time_stamp = *epoch_millis;
                        });

                        cx.shared.blink_led_config.lock(|config| {
                            config.blink_end_time = time_stamp + ((*duration_secs as i64)*1000);
                            //TODO: this would act funny after 1 kHz
                            config.blink_period_millis = 1000/freq_hz;
                        });
                    },
                    Message::D(udt, duration_secs, freq_hz) => {
                      rprintln!("Received Set({}, ([year={}, month={}, day={}, hour={}, min={}, sec={}, nsec={}], {} sec, {} Hz, {})", id, udt.year, udt.month, udt.day, udt.hour, udt.minute, udt.second, udt.nanoseconds, duration_secs, freq_hz, devid);
                      let dt = Utc.with_ymd_and_hms(udt.year, udt.month, udt.day, udt.hour, udt.minute, udt.second).unwrap();
          
                      let new_epoch_millis = dt.timestamp_millis();

                      cx.shared.epoch_millis.lock(|epoch_millis| {
                        *epoch_millis = new_epoch_millis;
                      });

                      cx.shared.blink_led_config.lock(|config| {
                        config.blink_end_time = new_epoch_millis + ((*duration_secs as i64)*1000);
                        //TODO: this would act funny after 1 kHz
                        config.blink_period_millis = 1000/freq_hz;
                      });                      
                    }
                    _ => {
                      rprintln!("[ERROR] - Set Message format not recognised!");
                    },
                  };
                },
                Command::Get(id, param, devid) => {
                  rprintln!("Received Get({},{},{})", id, param, devid);
                },
                _ => {
                  rprintln!("[ERROR] - Received cmd not recognised!");
                },
              };

              match sender.try_send(Response::SetOk) {
                Err(_) => {
                    rprintln!("send buffer full");
                }
                _ => {}
            }
              
            } else {
              
              *rx_idx = *rx_idx + 1;
            }
        }
        //rprintln!("");
        rx.reset_rx_fifo_full_interrupt()
    }

    #[task(priority = 1, local = [ tx ])]
    async fn uart_tx(cx: uart_tx::Context, mut receiver: Receiver<'static, Response, CAPACITY>) {
        
        rprintln!("uart_tx started");
        let tx = cx.local.tx;

        while let Ok(c) = receiver.recv().await {

            let mut tx_buff : OutBuf = [0; OUT_SIZE];

            match c {
              Response::SetOk => {
                rprintln!("Sending Response::SetOk");

              },
              Response::ParseError => {
                rprintln!("Sending Response::ParseError");
              },
              Response::Data(id, param, val, devid) => {
                rprintln!("Sending Response::Data({},{},{},{}", id, param, val, devid);
              }
            }

            let to_write = serialize_crc_cobs(&c, &mut tx_buff);
            tx.write_bytes(to_write).unwrap();
        }
    }

    fn get_led_color(epoch_millis : i64) -> RGB<u8> {
        let hours = Utc.timestamp_opt(epoch_millis/1000, 0).unwrap().hour();
        if hours >= 3 && hours < 9 {
            return RGB {r: 0xF8, g: 0xF3, b: 0x2B};
        } else if hours >= 9 && hours < 15 {
            return RGB {r: 0x9C, g: 0xFF, b: 0xFA};
        } else if hours >= 15 && hours < 21 {
            return RGB {r: 0x05, g: 0x3C, b: 0x5E};
        }
        return RGB {r: 0x31, g: 0x08, b: 0x1F};
    }

    // led blinking task
    #[task(binds = TG0_T0_LEVEL, shared = [tg0_timer0, blink_led], priority = 1)]
    fn blink(mut cx: blink::Context) {
        cx.shared.blink_led.lock(|led| {
            if led.is_set_high().unwrap() {
                led.set_low().unwrap();
            } else {
                led.set_high().unwrap();
            }
        });
        cx.shared.tg0_timer0.lock(|tg0_timer0| {
            tg0_timer0.clear_interrupt();
            tg0_timer0.set_alarm_active(true);
            tg0_timer0.listen();
        });
    }

    // We should not pre-empt this so that the wide time stamps are correct.
    #[task(binds = TG1_T0_LEVEL, local = [tg1_timer0, rtc, previous_rtc_timestamp, color_led],
        shared = [epoch_millis, blink_led_config, tg0_timer0, blink_led, color_led_active], priority = 2)]
    fn advance_time(mut cx: advance_time::Context) {
    
        let new_time : u64 = cx.local.rtc.get_time_ms();
        // Calculate time that has passed since last interrupt.
        let millis_passed : u64 = new_time - *cx.local.previous_rtc_timestamp;

        // Create a time stamp for this interrupt.
        *cx.local.previous_rtc_timestamp = new_time;

        let mut timestamp : i64 = 0;
        cx.shared.epoch_millis.lock(|epoch_millis| {
            *epoch_millis = *epoch_millis + (millis_passed as i64);
            timestamp = *epoch_millis;
        });

        let dt = Utc.timestamp_opt(timestamp/1000, 0).unwrap();
        rprintln!("[{}-{:02}-{:02} {:02}:{:02}:{:02}]", dt.year(), dt.month(), dt.day(),  dt.hour(), dt.minute(), dt.second());

        let mut end_blinking : bool = false;
        let mut start_blinking : bool = false;

        let mut blink_period : u32 = 0;


        // Check time values whether we should start or stop blinking
        cx.shared.blink_led_config.lock(|config| {
            if timestamp > config.blink_end_time && config.active {
                config.active = false;
                end_blinking = true;
                rprintln!("Ending blinking");
            } else if timestamp > config.blink_start_time && timestamp < config.blink_end_time  && !config.active {
                rprintln!("Starting blinking");
                start_blinking = true;
                config.active = true;
                blink_period = config.blink_period_millis/2;
            }
        });

        // Set the interrupt parameters for the timer that triggers blinking
        cx.shared.tg0_timer0.lock(|tg0_timer0| {
            if end_blinking {
                tg0_timer0.clear_interrupt();
                tg0_timer0.set_alarm_active(false);
                tg0_timer0.unlisten();
            } else if start_blinking {
                tg0_timer0.start(blink_period.millis());
                tg0_timer0.clear_interrupt();
                tg0_timer0.set_alarm_active(true);
                tg0_timer0.listen();
            }
        });

        cx.shared.blink_led.lock(|led| {
            // Make sure the led is switched off if we stop blinking
            if end_blinking {
                led.set_low().unwrap();
            }
        });

        // TODO: clean this mess up

        let mut color = RGB{r: 0, g: 0, b: 0};
        cx.shared.color_led_active.lock(|active| {
            if *active {
                color = get_led_color(timestamp);
            }
            cx.local.color_led.write(brightness([color].iter().cloned(), 10)).unwrap();
        });

        cx.local.tg1_timer0.clear_interrupt();
        cx.local.tg1_timer0.set_alarm_active(true);
    }
}
