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
        peripherals::{Peripherals, TIMG0, UART0},
        prelude::*,
        timer::{Timer, Timer0, TimerGroup},
        uart::{
            config::{Config, DataBits, Parity, StopBits},
            TxRxPins, UartRx, UartTx,
        },
        Uart, IO,
    };

    use rtic_sync::{channel::*, make_channel};
    use rtt_target::{rprint, rprintln, rtt_init_print};

    use core::mem::size_of;

    // shared libs
    use corncobs::{max_encoded_len, ZERO};
    use shared::{decode_command, serialize_crc_cobs, Command, Message, Response}; // local library

    const IN_SIZE: usize = max_encoded_len(size_of::<Command>() + size_of::<u32>());
    const OUT_SIZE: usize = max_encoded_len(size_of::<Response>() + size_of::<u32>());

    type InBuf = [u8; IN_SIZE];
    type OutBuf = [u8; OUT_SIZE];

    const CAPACITY: usize = 100;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        timer0: Timer<Timer0<TIMG0>>,
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

        let (tx, rx) = uart0.split();

        let rx_buff : InBuf = [0; IN_SIZE];
        let rx_idx  : usize = 0;

        uart_tx::spawn(receiver).unwrap();

        (
            Shared {},
            Local {
                timer0,
                tx,
                rx,
                sender,
                rx_buff,
                rx_idx,
            },
        )
    }

    // notice this is not an async task
    #[idle(local = [ timer0 ])]
    fn idle(cx: idle::Context) -> ! {
        loop {
            //rprintln!("idle, do some background work if any ...");
            // not async wait
            nb::block!(cx.local.timer0.wait()).unwrap();
        }
    }

    #[task(binds = UART0, priority=2, local = [ rx, sender, rx_buff, rx_idx])]
    fn uart0(cx: uart0::Context) {
        
        let rx = cx.local.rx;
        let sender = cx.local.sender;
        
        let rx_buff = cx.local.rx_buff;
        let rx_idx = cx.local.rx_idx;
        

        while let nb::Result::Ok(c) = rx.read() {

            rx_buff[*rx_idx] = c;
            //rprintln!("c = {}, idx = {}", c, rx_idx);

            // reset element idx counter when eof received
            if c == ZERO {
              
              *rx_idx = 0;

              let cmd = decode_command(rx_buff).unwrap();

              match &cmd {

                Command::Set(id, msg, devid) => {

                  match &msg {
                    Message::A(udt) => {
                      rprintln!("Received Set({}, [year={}, month={}, day={}, hour={}, min={}, sec={}, nsec={}],{})", id, udt.year, udt.month, udt.day, udt.hour, udt.minute, udt.second, udt.nanoseconds, devid);
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
}
