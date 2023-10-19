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
    use rtic_monotonics::{
        self,
        esp32c3_systimer::{ExtU64, Systimer, fugit::Instant},
        Monotonic,
    };

    use shared::shift_register::{ShiftRegister, self};

    //use ::{Duration, ExtU32};
    

    #[shared]
    struct Shared {
        old_ticks : Instant<u64, 1, 16000000>,
        timer0 : Timer<Timer0<TIMG0>>, 
    }

    #[local]
    struct Local {
        button: Gpio9<Input<PullUp>>,
        led: Gpio7<Output<PushPull>>,
        shift_reg: ShiftRegister,    
        shift_reg_count : u32,   
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        rtt_init_print!();
        rprintln!(env!("CARGO_CRATE_NAME"));

        let peripherals = Peripherals::take();
        let mut system = peripherals.SYSTEM.split();
        let clocks = ClockControl::max(system.clock_control).freeze();

        // configure TIMG0 to be used for global clock
        let timer_group0 = TimerGroup::new(
            peripherals.TIMG0,
            &clocks,
            &mut system.peripheral_clock_control,
        );
        let mut timer0 = timer_group0.timer0;
        timer0.clear_interrupt();

        let systimer_token = rtic_monotonics::create_systimer_token!();
        Systimer::start(cx.core.SYSTIMER, systimer_token);
        
        // It was empirically determined that Systimer::now() needs to be called 3 times in order for the System timer to actually function (sometimes). 
        let mut old_ticks = Systimer::now();
        old_ticks = Systimer::now();
        old_ticks = Systimer::now();
        rprintln!("Old ticks init value = {:?}", old_ticks.ticks());

        // configure button on GPIO9 (interrupt) and LED on GPIO7
        let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
        let mut led = io.pins.gpio7.into_push_pull_output();
        let mut button: esp32c3_hal::gpio::GpioPin<Input<PullUp>, 9> = io.pins.gpio9.into_pull_up_input();
        button.listen(esp32c3_hal::gpio::Event::FallingEdge);

        let mut shift_reg = ShiftRegister{reg:[0;3]};

        let mut shift_reg_count : u32 = 0;

        // initialise LED to low
        led.set_low().unwrap();

        #[allow(unreachable_code)]
        (Shared {old_ticks, timer0} , Local {led, button, shift_reg, shift_reg_count})
    }

        // notice this is not an async task
    #[idle(local = [])]
        fn idle(cx: idle::Context) -> ! {
        loop {
            // idle loop
        }
    }

    // button task to trigger whenever button is pressed. Updates led_switch to true whenever called
    #[task(binds = GPIO, local = [button, shift_reg, shift_reg_count], shared = [old_ticks, timer0], priority = 2)]
    fn button(mut cx: button::Context) {

        let mut ticks_now : Instant<u64, 1, 16000000> = Systimer::now();
        // Call this twice, otherwise timer does not update
        // TODO: Why?
        ticks_now = Systimer::now();

        cx.shared.old_ticks.lock(|old_ticks| {
            cx.local.shift_reg.insert(ticks_now.checked_duration_since(*old_ticks).unwrap().to_millis());
            rprintln!("inserted {:?}ms into shift reg", ticks_now.checked_duration_since(*old_ticks).unwrap().to_millis());
            
            if *cx.local.shift_reg_count < 3 {
                *cx.local.shift_reg_count = *cx.local.shift_reg_count + 1;
            }
            
            if ticks_now == *old_ticks {
                rprintln!("Error: Timer is acting funny!");
            } 
            
            *old_ticks = ticks_now;

        });
        
        if *cx.local.shift_reg_count >= 3 {
          cx.shared.timer0.lock(|timer0| {
              timer0.unlisten();
              timer0.reset_counter();
              timer0.start(cx.local.shift_reg.avg().millis());
              rprintln!("Average value is: {}ms", cx.local.shift_reg.avg());
              timer0.listen();
          });
        }
        
        cx.local.button.clear_interrupt();
    }

    // led blinking task
    #[task(binds = TG0_T0_LEVEL, local = [led], shared = [timer0], priority = 1)]
    fn blink(mut cx: blink::Context) {

        if cx.local.led.is_set_high().unwrap() {
            cx.local.led.set_low().unwrap();
        } else {
            cx.local.led.set_high().unwrap();
        }
        cx.shared.timer0.lock(|timer0| {
            timer0.clear_interrupt();
            timer0.set_alarm_active(true);
        });
    }
}
