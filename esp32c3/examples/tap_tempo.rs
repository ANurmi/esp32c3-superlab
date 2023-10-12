//! panic
//!
//! Run on target:
//!
//! cargo embed --example panic
//!
//! Showcases basic panic handling

#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

// bring in panic handler
use panic_rtt_target as _;

#[rtic::app(device = esp32c3, dispatchers = [FROM_CPU_INTR0, FROM_CPU_INTR1])]
mod app {
    use rtt_target::{rprintln, rtt_init_print};

    // to bring in interrupt vector initialization
    use esp32c3_hal::{
        self as _,
        clock::ClockControl,
        gpio::{Gpio9, Input, PullUp},
        gpio::{Gpio7, Output, PushPull},
        timer::{Timer, Timer0, TimerGroup},
        peripherals::{Peripherals, TIMG0},
        prelude::*,
        IO,
    };

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        timer0: Timer<Timer0<TIMG0>>,
        led: Gpio7<Output<PushPull>>,
        button: Gpio9<Input<PullUp>>,
    }

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        rtt_init_print!();
        rprintln!(env!("CARGO_CRATE_NAME"));

        let peripherals = Peripherals::take();
        let mut system = peripherals.SYSTEM.split();
        let clocks = ClockControl::max(system.clock_control).freeze();

        let timer_group0 = TimerGroup::new(
            peripherals.TIMG0,
            &clocks,
            &mut system.peripheral_clock_control,
        );
        let mut timer0 = timer_group0.timer0;
        timer0.start(1u64.secs());

        let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
        let mut led = io.pins.gpio7.into_push_pull_output();
        let mut button = io.pins.gpio9.into_pull_up_input();
        button.listen(esp32c3_hal::gpio::Event::FallingEdge);
        led.set_low().unwrap();


        #[allow(unreachable_code)]
        (Shared {}, Local { timer0, led, button })
    }

        // notice this is not an async task
    #[idle(local = [ timer0 ])]
        fn idle(cx: idle::Context) -> ! {
        loop {
            rprintln!("Timer fired!!!");
            // not async wait
            nb::block!(cx.local.timer0.wait()).unwrap();
        }
    }

    #[task(binds = GPIO, local = [button, led])]
    fn button(cx: button::Context) {
        cx.local.led.toggle().unwrap();
        rprintln!("button press");
        cx.local.button.clear_interrupt();
    }
}
