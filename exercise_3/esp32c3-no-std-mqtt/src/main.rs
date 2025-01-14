//! Run this:
//!
//! 1. Set SSID and PASSWORD environment variables to match with the WiFi connection you're using
//!   - `export SSID=my_wifi PASSWORD=my_pass`
//!   - Note that your wifi password is now stored in your environment.
//! 2.
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

// peripherals-related imports
use hal::{
    clock::{ClockControl, CpuClock},
    embassy,
    peripherals::{Peripherals, I2C0},
    prelude::*,
    systimer::SystemTimer,
    timer::TimerGroup,
    Rng,
    IO,
    i2c::I2C,
    Delay,
};

use shtcx::PowerMode;

// Wifi-related imports
use embedded_svc::wifi::{ClientConfiguration, Configuration, Wifi};
use esp_wifi::{
    wifi::{WifiController, WifiDevice, WifiEvent, WifiMode, WifiState},
    {initialize, EspWifiInitFor},
};

// embassy related imports
use embassy_executor::{Executor, _export::StaticCell};
use embassy_net::{
    tcp::TcpSocket,
    {dns::DnsQueryType, Config, Stack, StackResources},
};
use embassy_time::{Duration, Timer};

// MQTT related imports
use mqtt_topics::{temperature_data_topic, Esp, humidity_data_topic};
use rust_mqtt::{
    client::{client::MqttClient, client_config::ClientConfig},
    packet::v5::reason_codes::ReasonCode,
    utils::rng_generator::CountingRng,
};

use esp_backtrace as _;
use esp_println::println;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

// SELECT WHICH DEVICE YOU ARE PROGRAMMING BY UNCOMMENTING THE UUID BELOW
const UUID: &str = "16e337a0-935d-4f32-bf3c-6ded006cesp0";
//const UUID: &str = "16e337a0-935d-4f32-bf3c-6ded006cesp1";
//const UUID: &str = "16e337a0-935d-4f32-bf3c-6ded006cesp2";

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

macro_rules! singleton {
    ($val:expr) => {{
        type T = impl Sized;
        static STATIC_CELL: StaticCell<T> = StaticCell::new();
        let (x,) = STATIC_CELL.init(($val,));
        x
    }};
}

// maintains wifi connection, when it disconnects it tries to reconnect
#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.get_capabilities());
    loop {
        match esp_wifi::wifi::get_wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.into(),
                password: PASSWORD.into(),
                ..Default::default()
            });

            match controller.set_configuration(&client_config) {
                Ok(()) => {}
                Err(e) => {
                    println!("Failed to connect to wifi: {e:?}");
                    continue;
                }
            }
            println!("Starting wifi");
            match controller.start().await {
                Ok(()) => {}
                Err(e) => {
                    println!("Failed to connect to wifi: {e:?}");
                    continue;
                }
            }
            println!("Wifi started!");
        }
        println!("About to connect...");

        match controller.connect().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

// A background task, to process network events - when new packets, they need to processed, embassy-net, wraps smoltcp
#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static>>) {
    stack.run().await
}

// our "main" task
#[embassy_executor::task]
async fn task(
    stack: &'static Stack<WifiDevice<'static>>,
    i2c: I2C<'static, I2C0>,
    mut delay: Delay
) {
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    //wait until wifi connected
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            println!("Got IP: {}", config.address); //dhcp IP address
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    loop {
        Timer::after(Duration::from_millis(1_000)).await;

        let mut socket = TcpSocket::new(&stack, &mut rx_buffer, &mut tx_buffer);

        socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

        let address = match stack
            .dns_query("test.mosquitto.org", DnsQueryType::A)
            .await
            .map(|a| a[0])
        {
            Ok(address) => address,
            Err(e) => {
                println!("DNS lookup error: {e:?}");
                continue;
            }
        };

        let remote_endpoint = (address, 1883);
        println!("connecting...");
        let connection = socket.connect(remote_endpoint).await;
        if let Err(e) = connection {
            println!("connect error: {:?}", e);
            continue;
        }
        println!("connected!");

        let mut config = ClientConfig::new(
            rust_mqtt::client::client_config::MqttVersion::MQTTv5,
            CountingRng(20000),
        );
        config.add_max_subscribe_qos(rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS0);
        config.max_packet_size = 100;
        let mut recv_buffer = [0; 80];
        let mut write_buffer = [0; 80];

        let mut client =
            MqttClient::<_, 5, _>::new(socket, &mut write_buffer, 80, &mut recv_buffer, 80, config);

        match client.connect_to_broker().await {
            Ok(()) => {}
            Err(mqtt_error) => match mqtt_error {
                ReasonCode::NetworkError => {
                    println!("MQTT Network Error");
                    continue;
                }
                _ => {
                    println!("Other MQTT Error: {:?}", mqtt_error);
                    continue;
                }
            },
        }

        let mut shtcx = shtcx::shtc3(i2c);

        loop {

            let measurement = shtcx.measure(PowerMode::NormalMode, &mut delay).unwrap();
            let temperature_celsius = measurement.temperature.as_degrees_celsius();
            let humidity = measurement.humidity.as_percent();

            match client
                .send_message(
                    temperature_data_topic(UUID, Esp::EspTarget1).as_str(),
                    &temperature_celsius.to_be_bytes() as &[u8],
                    rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS0,
                    true,
                )
                .await
            {
                Ok(()) => {
                    println!("Send Temperature OK : UUID = {UUID}")
                }
                Err(mqtt_error) => match mqtt_error {
                    ReasonCode::NetworkError => {
                        println!("MQTT Network Error");
                        continue;
                    }
                    _ => {
                        println!("Other MQTT Error: {:?}", mqtt_error);
                        continue;
                    }
                },
            }

            match client
                .send_message(
                    humidity_data_topic(UUID, Esp::EspTarget1).as_str(),
                    &humidity.to_be_bytes() as &[u8],
                    rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS0,
                    true,
                )
                .await
            {
                Ok(()) => {
                    println!("Send Humidity OK : UUID = {UUID}")
                }
                Err(mqtt_error) => match mqtt_error {
                    ReasonCode::NetworkError => {
                        println!("MQTT Network Error");
                        continue;
                    }
                    _ => {
                        println!("Other MQTT Error: {:?}", mqtt_error);
                        continue;
                    }
                },
            }
            Timer::after(Duration::from_millis(1000)).await;
        }
    }
}

#[entry]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();

    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);

    let delay = Delay::new(&clocks);
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);        

    let init = initialize(
        EspWifiInitFor::Wifi,
        SystemTimer::new(peripherals.SYSTIMER).alarm0,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .expect("Failed to initialize Wifi");

    embassy::init(&clocks, timer_group0.timer0);

    let wifi = peripherals.WIFI;
    let (wifi_interface, controller) =
        match esp_wifi::wifi::new_with_mode(&init, wifi, WifiMode::Sta) {
            Ok((wifi_interface, controller)) => (wifi_interface, controller),
            Err(..) => panic!("WiFi mode Error!"),
        };

    let config = Config::dhcpv4(Default::default());

    let seed = 1234; // very random, very secure seed

    let i2c0 = I2C::new(
        peripherals.I2C0,
        io.pins.gpio10,
        io.pins.gpio8,
        300u32.kHz(),
        &clocks
    );

    // Init network stack
    let stack = &*singleton!(Stack::new(
        wifi_interface,
        config,
        singleton!(StackResources::<3>::new()),
        seed
    ));

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(connection(controller)).unwrap();
        spawner.spawn(net_task(&stack)).unwrap();
        spawner.spawn(task(&stack, i2c0, delay)).unwrap();
        //spawner.spawn(print_temperature(&i2c0, delay)).unwrap();
    });
}

pub async fn sleep(millis: u32) {
    Timer::after(Duration::from_millis(millis as u64)).await;
}
