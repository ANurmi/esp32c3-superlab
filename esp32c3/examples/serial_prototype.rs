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

use panic_rtt_target as _;

// bring in panic handler
use panic_rtt_target as _;

#[rtic::app(device = esp32c3, dispatchers = [FROM_CPU_INTR0, FROM_CPU_INTR1])]
mod app {
    use esp32c3_hal::{
        clock::ClockControl,
        peripherals::{Peripherals, TIMG0, TIMG1, UART0},
        prelude::*,
        timer::{Timer, Timer0, TimerGroup},
        uart::{
            config::{Config, DataBits, Parity, StopBits},
            TxRxPins, UartRx, UartTx,
        },
        Rtc,
        Uart, IO,
    };

    use rtic_sync::{channel::*, make_channel};
    use rtt_target::{rprint, rprintln, rtt_init_print};

    use core::mem::size_of;

    use core::time::Duration;
    use chrono::prelude::*;

    use chrono::{Utc};

    // shared libs
    use corncobs::{max_encoded_len, ZERO};
    use shared::{deserialize_crc_cobs, serialize_crc_cobs, Command, Message, Response, Faults}; // local library

    const IN_SIZE: usize = max_encoded_len(size_of::<Command>() + size_of::<u32>());
    const OUT_SIZE: usize = max_encoded_len(size_of::<Response>() + size_of::<u32>());

    type InBuf = [u8; IN_SIZE];
    type OutBuf = [u8; OUT_SIZE];

    const CAPACITY: usize = 100;

    #[shared]
    struct Shared {
      epoch_millis : i64,
    }

    #[local]
    struct Local {
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
        let mut timer0 = timer_group0.timer0;

        let timer_group1 = TimerGroup::new(
          peripherals.TIMG1,
          &clocks,
          &mut system.peripheral_clock_control,
        );
        let mut tg1_timer0 = timer_group1.timer0;
        timer0.clear_interrupt();
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

        timer0.start(1u64.secs());
        tg1_timer0.start(1u64.secs());

        let (tx, rx) = uart0.split();

        let rx_buff : InBuf = [0; IN_SIZE];
        let rx_idx  : usize = 0;

        let rtc = Rtc::new(peripherals.RTC_CNTL);
        

        // SET THIS WITH UART
        let datetime = Utc.ymd(2023, 1, 1).and_hms(0, 0, 0);
          
        let epoch_millis = datetime.timestamp_millis();

        let previous_rtc_timestamp = rtc.get_time_ms();


        uart_tx::spawn(receiver).unwrap();

        tg1_timer0.listen();

        (
            Shared {
              epoch_millis,
            },
            Local {
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
    fn idle(cx: idle::Context) -> ! {
        loop {
            //rprintln!("idle, do some background work if any ...");
            // not async wait
            //nb::block!(cx.local.timer0.wait()).unwrap();
        }
    }

    #[task(binds = UART0, priority=2, local = [ rx, sender, rx_buff, rx_idx], shared = [epoch_millis])]
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

              let cmd_res = deserialize_crc_cobs(rx_buff);
              let mut rsp = Response::SetOk;
              
              match cmd_res {

                // extract command if no errors were identified during the deserialise process
                Ok(cmd) => {

                  match cmd {

                    Command::Set(id, msg, devid) => {
                      
                      // 
                      match msg {

                        Message::A(udt) => {
                          rprintln!("Received Set({}, [year={}, month={}, day={}, hour={}, min={}, sec={}, nsec={}],{})", id, udt.year, udt.month, udt.day, udt.hour, udt.minute, udt.second, udt.nanoseconds, devid);
                          let datetime = Utc.ymd(udt.year, udt.month, udt.day).and_hms(udt.hour, udt.minute, udt.second);
              
                          let new_epoch_millis = datetime.timestamp_millis();
    
                          cx.shared.epoch_millis.lock(|epoch_millis| {
                            *epoch_millis = new_epoch_millis;
                          });
                        },
                        Message::B(int_val) => {
                          rprintln!("Received Set({},{},{})", id, int_val, devid);
                        },
                        Message::C(duration_secs, freq_hz) => {
                          rprintln!("Received Set({},({} sec, {} Hz),{})", id, duration_secs, freq_hz, devid);
                        },
                        Message::D(udt, duration_secs, freq_hz) => {
                          rprintln!("Received Set({}, ([year={}, month={}, day={}, hour={}, min={}, sec={}, nsec={}], {} sec, {} Hz, {})", id, udt.year, udt.month, udt.day, udt.hour, udt.minute, udt.second, udt.nanoseconds, duration_secs, freq_hz, devid);
                        }
                        _ => {
                          rprintln!("[ERROR] - Set Message format not recognised!");
                          rsp = Response::Illegal;
                        },
                      };
                    },

                    Command::Get(id, param, devid) => {
                      rprintln!("Received Get({},{},{})", id, param, devid);
                    },

                  };
                },
                // Use the error reported in the serialise process to determine how to respond
                Err(fault) => {
    
                  match fault {
    
                    Faults::BitFlipData => { 
                      rprintln!("Detected bitflip in payload or CRC!");
                      rsp = Response::NotOK;
                    },
                    _ => {
                      rprintln!("[ERROR] - Received cmd not recognised!");
                      rsp = Response::NotOK;
                    },
                  };
                }
              };

              match sender.try_send(rsp) {
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
              },
              Response::NotOK => {
                rprint!("Sending Response::NotOK");
              },
              Response::Illegal => {
                rprint!("Sending Response::Illegal");
              },
            }

            let to_write = serialize_crc_cobs(&c, &mut tx_buff);
            tx.write_bytes(to_write).unwrap();
        }
    }

    #[task(binds = TG1_T0_LEVEL, local = [tg1_timer0, rtc, previous_rtc_timestamp], shared = [epoch_millis], priority = 2)]
    fn advance_time(mut cx: advance_time::Context) {
    
        let new_time : u64 = cx.local.rtc.get_time_ms();
        // Calculate time that has passed since last interrupt.
        let millis_passed : u64 = new_time - *cx.local.previous_rtc_timestamp;

        // Create a time stamp for this interrupt.
        *cx.local.previous_rtc_timestamp = new_time;

        // TODO: can we get this in seconds somewhere?
        let mut timestamp : i64 = 0;
        cx.shared.epoch_millis.lock(|epoch_millis| {
            *epoch_millis = *epoch_millis + (millis_passed as i64);
            timestamp = *epoch_millis;
        });
        rprintln!("epoch in ms is {}", timestamp);

        let dt = Utc.timestamp(timestamp/1000, 0);
        rprintln!("{}-{}-{} {}:{}:{}", dt.year(), dt.month(), dt.day(),  dt.hour(), dt.minute(), dt.second());

        cx.local.tg1_timer0.clear_interrupt();
        cx.local.tg1_timer0.set_alarm_active(true);
    }
}
