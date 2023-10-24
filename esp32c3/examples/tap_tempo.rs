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
        timer::{Timer, Wdt, Timer0, TimerGroup},
        peripherals::{Peripherals, TIMG0},
        prelude::*,
        systimer::SystemTimer,
        IO,
    };
    use rtic_monotonics::{
        self,
        esp32c3_systimer::{ExtU64, Systimer},
    };

    use shared::shift_register::ShiftRegister;
    

    #[shared]
    struct Shared {
        timer0 : Timer<Timer0<TIMG0>>, 
    }

    #[local]
    struct Local {
        button: Gpio9<Input<PullUp>>,
        led: Gpio7<Output<PushPull>>,
        shift_reg: ShiftRegister,
        old_ticks: u64,
        wdt0: Wdt<TIMG0>,
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
        let mut wdt0 = timer_group0.wdt;
        timer0.clear_interrupt();

        wdt0.start(10u64.secs());

        let systimer_token = rtic_monotonics::create_systimer_token!();
        Systimer::start(cx.core.SYSTIMER, systimer_token);

        let _syst = SystemTimer::new(peripherals.SYSTIMER);

        let old_ticks = SystemTimer::now();

        // configure button on GPIO9 (interrupt) and LED on GPIO7
        let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
        let mut led = io.pins.gpio7.into_push_pull_output();
        let mut button: esp32c3_hal::gpio::GpioPin<Input<PullUp>, 9> = io.pins.gpio9.into_pull_up_input();
        button.listen(esp32c3_hal::gpio::Event::FallingEdge);

        let shift_reg = ShiftRegister::new();

        // initialise LED to low
        led.set_low().unwrap();

        rprintln!("Init Called!");

        #[allow(unreachable_code)]
        (Shared {
            timer0
        } , Local {
            led,
            button, 
            shift_reg,
            old_ticks,
            wdt0,
        })
    }

    #[idle(local = [])]
        fn idle(_: idle::Context) -> ! {
        loop {
            // idle loop
        }
    }

    // button task to trigger whenever button is pressed. Updates led_switch to true whenever called
    #[task(binds = GPIO, local = [button, shift_reg, old_ticks, wdt0], shared = [timer0], priority = 2)]
    fn button(mut cx: button::Context) {

        rprintln!("Feeding Watchdog!");
        // feed the watchdog
        cx.local.wdt0.feed();

        let new_ticks = SystemTimer::now();
        // convert ticks to ms by div by 16,384 (approximately correct but more efficient than accurate division of 16,000)
        // divide by two to ensure on -> off is written to shift reg
        let duration_ms = ((new_ticks - *cx.local.old_ticks) >> 14) >> 1;
        

        cx.local.shift_reg.insert(duration_ms);
        rprintln!("inserted {:?}ms into shift reg", duration_ms);
        
        if new_ticks == *cx.local.old_ticks {
            rprintln!("Error: Timer is acting funny!");
        } 
        
        *cx.local.old_ticks = new_ticks;

        if cx.local.shift_reg.valid_entries() {
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
