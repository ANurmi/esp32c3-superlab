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
        peripherals::{Peripherals, TIMG0, TIMG1},
        prelude::*,
        IO,
    };
    // struct to contain RTC information used for global counter
    pub struct RTC {
        millis : u64,
        secs : u64,
        mins : u64, 
        hours : u64,
    }

    // methods to access global clock information
    impl RTC {
        pub fn get_time_millis(&self) -> u64 {
            self.millis + (1000*self.secs) + (60 * 1000 * self.mins) + (60* 60 * 1000 * self.hours)
        }
        pub fn print_time(&self) -> () {
            rprintln!("{:02}:{:02}:{:02}.{:03}", self.hours, self.mins, self.secs, self.millis);
        }
        pub fn increment_millis(&mut self, incr : u64) -> () {
            self.millis = self.millis + incr;
            if self.millis >= 1000 {
                self.millis = self.millis - 1000;
                if self.secs == 59 {
                    self.secs = 0;
                    if self.mins == 59 {
                        self.mins = 0;
                        self.hours = self.hours + 1;
                    } else {
                        self.mins = self.mins + 1;
                    }
                } else {
                    self.secs = self.secs + 1;
                }
            }
        }
        pub fn reset(&mut self) -> () {
            self.millis = 0;
            self.secs = 0;
            self.mins = 0;
            self.hours = 0;
        }
    }

    const TIMER_UPDATE_PERIOD_MS : u64 = 10;

    #[shared]
    struct Shared {
        led_switch : bool,
        global_time : RTC,
    }

    #[local]
    struct Local {
        timer0: Timer<Timer0<TIMG0>>,
        led: Gpio7<Output<PushPull>>,
        button: Gpio9<Input<PullUp>>,
        timer1: Timer<Timer0<TIMG1>>,
        led_period_millis: u64,        
    }

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
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
        timer0.start(TIMER_UPDATE_PERIOD_MS.millis());
        timer0.listen();       

        // configure TIMG1 to be used for LED clock
        let timer_group1 = TimerGroup::new(
            peripherals.TIMG1,
            &clocks,
            &mut system.peripheral_clock_control,
        ); 
        let mut led_period_millis : u64 = 500;
        let mut timer1 = timer_group1.timer0; 
        timer1.clear_interrupt();
        timer1.start(led_period_millis.millis());
        timer1.listen(); 
        
        // configure button on GPIO9 (interrupt) and LED on GPIO7
        let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
        let mut led = io.pins.gpio7.into_push_pull_output();
        let mut button = io.pins.gpio9.into_pull_up_input();
        button.listen(esp32c3_hal::gpio::Event::FallingEdge);

        // initialise LED to low
        led.set_low().unwrap();

        // switch to control LED rate
        let mut led_switch = false;
        // instance of RTC to be used for global clock
        let mut global_time = RTC {millis : 0, secs : 0, mins : 0, hours : 0};

        #[allow(unreachable_code)]
        (Shared {led_switch, global_time}, Local { timer0, timer1, led, button, led_period_millis})
    }

        // notice this is not an async task
    #[idle(local = [])]
        fn idle(cx: idle::Context) -> ! {
        loop {
            // idle loop
        }
    }

    // button task to trigger whenever button is pressed. Updates led_switch to true whenever called
    #[task(binds = GPIO, local = [button], shared = [global_time, led_switch], priority = 3)]
    fn button(mut cx: button::Context) {
        rprintln!("button pressed at:");
        cx.shared.global_time.lock(|global_time| {
            global_time.print_time();
        }); 
        cx.shared.led_switch.lock(|led_switch| {
            *led_switch = true; 
        });
        cx.local.button.clear_interrupt();
    }

    // led blinking task, currently blinks at a rate determined by led_period_millis. If led_switch is set (modified in button task),
    // it updates the frequency of blinking
    #[task(binds = TG1_T0_LEVEL, local = [led, timer1, led_period_millis], shared = [led_switch], priority = 1)]
    fn blink(mut cx: blink::Context) {
        if cx.local.led.is_set_high().unwrap() {
            cx.local.led.set_low().unwrap();
        } else {
            cx.local.led.set_high().unwrap();
        }

        cx.shared.led_switch.lock(|led_switch| {
            if *led_switch == true {
                if *cx.local.led_period_millis == 500 {
                    *cx.local.led_period_millis = 250;
                } else {
                    *cx.local.led_period_millis = 500;
                }
                // set_alarm_active is implicitly called within start(), so no need to call it explicitly here
                cx.local.timer1.start(cx.local.led_period_millis.millis());
                *led_switch = false;
            } else {                
                cx.local.timer1.set_alarm_active(true);
            }
        });
        cx.local.timer1.clear_interrupt();
    }

    // global timer task to be scheduled periodically whenever TIMER_UPDATE_PERIOD_MS has elapsed
    #[task(binds = TG0_T0_LEVEL, local = [timer0], shared = [global_time], priority = 2)]
    fn timer(mut cx: timer::Context) {
        cx.shared.global_time.lock(|global_time| {
            global_time.increment_millis(TIMER_UPDATE_PERIOD_MS);
        }); 
        cx.local.timer0.clear_interrupt();
        cx.local.timer0.set_alarm_active(true);
    }
}
