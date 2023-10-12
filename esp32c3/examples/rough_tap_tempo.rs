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

#[rtic::app(device = esp32c3, dispatchers = [FROM_CPU_INTR0])]
mod app {
    use rtt_target::{rprintln, rtt_init_print};

    // to bring in interrupt vector initialization
    use esp32c3_hal::{
        self as _,
        clock::ClockControl,
        gpio::{Gpio9, Input, PullUp},
        gpio::{Gpio7, Output, PushPull},
        peripherals::Peripherals,
        prelude::*,
        IO, systimer::SystemTimer,
    };

    use rtic_monotonics::{
        self,
        esp32c3_systimer::{ExtU64, Systimer},
    };

    #[shared]
    struct Shared {
      button_pressed : bool,
    }

    #[local]
    struct Local {
        button: Gpio9<Input<PullUp>>,
        led: Gpio7<Output<PushPull>>,
        counter: u32,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        rtt_init_print!();
        rprintln!(env!("CARGO_CRATE_NAME"));

        let peripherals = Peripherals::take();
        let system = peripherals.SYSTEM.split();
        let _ = ClockControl::max(system.clock_control).freeze();

        let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
        let mut button = io.pins.gpio9.into_pull_up_input();
        let mut led = io.pins.gpio7.into_push_pull_output();

        // setup for monotonic timer
        let systimer_token = rtic_monotonics::create_systimer_token!();
        Systimer::start(cx.core.SYSTIMER, systimer_token);
        
        button.listen(esp32c3_hal::gpio::Event::AnyEdge);
        led.set_low().unwrap();

        let counter : u32 = 0; 

        timer_loop::spawn().unwrap();

        #[allow(unreachable_code)]
        (Shared { button_pressed : false, }, Local { button, led, counter })
    }

    #[idle(local = [led], shared = [button_pressed])]
    fn idle(mut cx: idle::Context) -> ! {
        loop {
          cx.shared.button_pressed.lock(|button_pressed| {
              if *button_pressed == true {
                  cx.local.led.set_high().unwrap();
              } else {
                  cx.local.led.set_low().unwrap();
              }
          });
        }
    }

    // loop which uses monotonic timer and prints every 2 secs
    #[task(priority = 1)]
    async fn timer_loop(_cx: timer_loop::Context) {
        loop {
          rprintln!("Timer loop");
          Systimer::delay(ExtU64::secs(2)).await;
        }
    }

    #[task(binds = GPIO, local = [button, counter], shared = [button_pressed])]
    fn button(mut cx: button::Context) {
        // if button is low (button is pressed)
        if cx.local.button.is_low().unwrap() {
          *cx.local.counter = *cx.local.counter + 1;
          rprintln!("button press");
          rprintln!("counter = {}", cx.local.counter);
          // write to shared button pressed
          cx.shared.button_pressed.lock(|button_pressed| {
            *button_pressed = true;
            rprintln!("button_pressed = {}", *button_pressed);
          });
          // if button is released (button is pressed)
        } else {
          rprintln!("button release");
          // clear to shared button pressed
          cx.shared.button_pressed.lock(|button_pressed| {
            *button_pressed = false;
            rprintln!("button_pressed = {}", *button_pressed);
          });
        }
        cx.local.button.clear_interrupt();
    }
}
