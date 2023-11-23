//! examples/async-delay.rs

#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

use panic_rtt_target as _;
#[rtic::app(device = esp32c3, dispatchers = [])]
mod app {
    use esp32c3_hal::{self as _, system};
    use rtic_monotonics::{
        self,
        esp32c3_systimer::{ExtU64, Systimer},
        Monotonic,
    };
    use rtt_target::{rprintln, rtt_init_print};

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        //time : 
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        rtt_init_print!();
        rprintln!("init");

        let systimer_token = rtic_monotonics::create_systimer_token!();
        Systimer::start(cx.core.SYSTIMER, systimer_token);

        foo::spawn().unwrap();
        bar::spawn().unwrap();
        baz::spawn().unwrap();

        (Shared {}, Local {})
    }

    #[task]
    async fn foo(_cx: foo::Context) {
        rprintln!("hello from foo");
        
        rprintln!("Waiting 1 Second...");
        let mut ticks_a = Systimer::now();
        Systimer::delay(1.secs()).await;
        let mut ticks_b = Systimer::now();
        rprintln!("Milliseconds past = {:?}", ticks_b.checked_duration_since(ticks_a).unwrap().to_millis());
        
        rprintln!("Waiting 10 Milliseconds...");
        ticks_a = Systimer::now();
        Systimer::delay(10.millis()).await;
        ticks_b = Systimer::now();
        rprintln!("Milliseconds past = {:?}", ticks_b.checked_duration_since(ticks_a).unwrap().to_millis());
        rprintln!("bye from foo");
    }

    #[task]
    async fn bar(_cx: bar::Context) {
        rprintln!("hello from bar");
        Systimer::delay(3.secs()).await;
        rprintln!("bye from bar");
    }

    #[task]
    async fn baz(_cx: baz::Context) {
        rprintln!("hello from baz");
        Systimer::delay(4.secs()).await;
        rprintln!("bye from baz");
    }
}
