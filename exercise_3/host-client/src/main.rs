use mqtt_topics::{temperature_data_topic, Esp, humidity_data_topic};
use rumqttc::{Client, MqttOptions, Packet, QoS};
use std::error::Error;
use std::time::{SystemTime, Duration};

const UUID: &'static str = get_uuid::uuid();
// define static UUID values to make it easier to configure multiple devices
// note that these are the values that will need to be used in the device firmware
const UUID_ESP0: &'static str = "16e337a0-935d-4f32-bf3c-6ded006cesp0";
const UUID_ESP1: &'static str = "16e337a0-935d-4f32-bf3c-6ded006cesp1";
const UUID_ESP2: &'static str = "16e337a0-935d-4f32-bf3c-6ded006cesp2";
// number of devices used to size data structures
const DEVICE_COUNT : usize = 3;
const DEVICE_TIMEOUT : Duration = Duration::from_secs(5);

#[derive(Debug)]
enum DeviceStatus {
    OK,
    DEAD
}

#[derive(Debug)]
// position of sensor
enum DevicePosition {
    Front,
    Top,
    Rear
}

struct Device {
    id: u32,
    uuid: String,
    position: DevicePosition,
    status: DeviceStatus,
    temperature: (f32, SystemTime), /* value, local time of value acquisition */
    humidity: (f32, SystemTime),
    valid: bool,
}

impl Device {

    fn print(&self, start_time: &SystemTime) ->() {
        println!("id = {:?}", self.id);
        println!("uuid = {:?}", self.uuid);
        println!("position = {:?}", self.position);
        println!("status = {:?}", self.status);
        println!("temperature = ({:.3}°C @ {:.3} secs),", 
            self.temperature.0, 
            self.temperature.1.duration_since(*start_time).unwrap().as_secs_f32()
        );
        println!("humidity    = ({:.3} % @ {:.3} secs)", 
            self.humidity.0, 
            self.humidity.1.duration_since(*start_time).unwrap().as_secs_f32()
        );
        println!("valid = {:?}\n", self.valid);
    }
}


fn main() -> Result<(), Box<dyn Error>> {

    let start_time = SystemTime::now();

    let client_id = UUID;
    let mqtt_host = "test.mosquitto.org";
    dbg!(UUID);

    let mut mqttoptions = MqttOptions::new(client_id, mqtt_host, 1883);
    mqttoptions.set_keep_alive(DEVICE_TIMEOUT);

    // establish MQTT subscriptions for temperature and humidity for 3 sensors
    let (mut client, mut connection) = Client::new(mqttoptions, 100);

    client.subscribe(
        temperature_data_topic(UUID_ESP0, Esp::EspTarget1).as_str(),
        QoS::AtMostOnce,
    )?;
    client.subscribe(
        humidity_data_topic(UUID_ESP0, Esp::EspTarget1).as_str(),
        QoS::AtMostOnce,
    )?;
    client.subscribe(
        temperature_data_topic(UUID_ESP1, Esp::EspTarget1).as_str(),
        QoS::AtMostOnce,
    )?;
    client.subscribe(
        humidity_data_topic(UUID_ESP1, Esp::EspTarget1).as_str(),
        QoS::AtMostOnce,
    )?;
    client.subscribe(
        temperature_data_topic(UUID_ESP2, Esp::EspTarget1).as_str(),
        QoS::AtMostOnce,
    )?;
    client.subscribe(
        humidity_data_topic(UUID_ESP2, Esp::EspTarget1).as_str(),
        QoS::AtMostOnce,
    )?;

    println!("[{:.3}] Initialising devices...", start_time.elapsed().unwrap().as_secs_f32());

    let mut devices = initialise_devices();
    report_system_status(&mut devices, &start_time);

    // Iterate to poll the eventloop for connection progress
    for (_, notification) in connection.iter().enumerate() {

        // if you want to see *everything*, uncomment:
        // println!("Notification = {:#?}", notification);
        if let Ok(rumqttc::Event::Incoming(Packet::Publish(publish_data))) = notification {
            
            if publish_data.topic == temperature_data_topic(UUID_ESP0, Esp::EspTarget1).as_str() {
                
                let data: &[u8] = &publish_data.payload;               
                
                let data: Result<[u8; 4], _> = data.try_into();

                if let Ok(data) = data {
                    // if device has published data, it is not dead
                    devices[0].status = DeviceStatus::OK;
                    // update value and timestamp of reported value
                    devices[0].temperature = (f32::from_be_bytes(data), SystemTime::now());
                }

            }

            if publish_data.topic == humidity_data_topic(UUID_ESP0, Esp::EspTarget1).as_str() {
                
                let data: &[u8] = &publish_data.payload;               
                
                let data: Result<[u8; 4], _> = data.try_into();

                if let Ok(data) = data {
                    // if device has published data, it is not dead
                    devices[0].status = DeviceStatus::OK;
                    // update value and timestamp of reported value
                    devices[0].humidity = (f32::from_be_bytes(data), SystemTime::now());
                }
            }

            if publish_data.topic == temperature_data_topic(UUID_ESP1, Esp::EspTarget1).as_str() {
                
                let data: &[u8] = &publish_data.payload;               
                
                let data: Result<[u8; 4], _> = data.try_into();

                if let Ok(data) = data {
                    // if device has published data, it is not dead
                    devices[1].status = DeviceStatus::OK;
                    // update value and timestamp of reported value
                    devices[1].temperature = (f32::from_be_bytes(data), SystemTime::now());
                }
            }

            if publish_data.topic == humidity_data_topic(UUID_ESP1, Esp::EspTarget1).as_str() {
                
                let data: &[u8] = &publish_data.payload;               
                
                let data: Result<[u8; 4], _> = data.try_into();

                if let Ok(data) = data {
                    // if device has published data, it is not dead
                    devices[1].status = DeviceStatus::OK;
                    // update value and timestamp of reported value
                    devices[1].humidity = (f32::from_be_bytes(data), SystemTime::now());
                }                
            }

            if publish_data.topic == temperature_data_topic(UUID_ESP2, Esp::EspTarget1).as_str() {
                
                let data: &[u8] = &publish_data.payload;               
                
                let data: Result<[u8; 4], _> = data.try_into();

                if let Ok(data) = data {
                    // if device has published data, it is not dead
                    devices[2].status = DeviceStatus::OK;
                    // update value and timestamp of reported value
                    devices[2].temperature = (f32::from_be_bytes(data), SystemTime::now());
                }
            }

            if publish_data.topic == humidity_data_topic(UUID_ESP2, Esp::EspTarget1).as_str() {
                
                let data: &[u8] = &publish_data.payload;               
                
                let data: Result<[u8; 4], _> = data.try_into();

                if let Ok(data) = data {
                    // if device has published data, it is not dead
                    devices[2].status = DeviceStatus::OK;
                    // update value and timestamp of reported value
                    devices[2].humidity = (f32::from_be_bytes(data), SystemTime::now());
                }
            }
        }        

        set_device_valid(&mut devices[0]);
        set_device_valid(&mut devices[1]);
        set_device_valid(&mut devices[2]);

        // to prevent previously buffered MQTT messages from being printed, iterate through
        // initialisation loop
        if SystemTime::now().duration_since(start_time).unwrap() < DEVICE_TIMEOUT {
            // initialise devices for timeout
            devices = initialise_devices();
        } else {
            report_system_status(&mut devices, &start_time);
        }        
    }
    Ok(())
}

/* Initialise all devices to dead and no updated values */
fn initialise_devices() -> [Device; DEVICE_COUNT] {

    let device0 = Device {
        id: 0u32,
        uuid: UUID_ESP0.to_string(),
        position: DevicePosition::Rear,
        status: DeviceStatus::DEAD,
        temperature: (0., SystemTime::UNIX_EPOCH),
        humidity: (0., SystemTime::UNIX_EPOCH),
        valid: false,
    };

    let device1 = Device {
        id: 1u32,
        uuid: UUID_ESP1.to_string(),
        position: DevicePosition::Top,
        status: DeviceStatus::DEAD,
        temperature: (0., SystemTime::UNIX_EPOCH),
        humidity: (0., SystemTime::UNIX_EPOCH),
        valid: false,
    };

    let device2 = Device {
        id: 2u32,
        uuid: UUID_ESP2.to_string(),
        position: DevicePosition::Front,
        status: DeviceStatus::DEAD,
        temperature: (0., SystemTime::UNIX_EPOCH),
        humidity: (0., SystemTime::UNIX_EPOCH),
        valid: false,
    };

    let device_array : [Device; DEVICE_COUNT] = [device0, device1, device2];
    device_array
}

// return selected device in priority order
// priority is set using the position as it correlates with reliability
// order is (highest priority) REAR->TOP->FRONT (lowest priority) 
fn get_select_device(
    devices: &mut [Device;DEVICE_COUNT]
) -> usize {

    if devices[0].valid {
        devices[0].id as usize
    } else if devices[1].valid {
        devices[1].id as usize
    } else if devices[2].valid {
        devices[2].id as usize
    } else {
        usize::MAX
    }
}

// report the status of the selected device
// if no device is valid, report so
fn report_system_status(
    devices: &mut [Device;DEVICE_COUNT],
    start_time: &SystemTime
) -> () {

    let selected_device = get_select_device(devices);
    let elapsed = start_time.elapsed().unwrap().as_secs_f32();

    if selected_device == usize::MAX {
        println!("[{:.3}] No valid devices!", elapsed);
    } else {
        println!("[{:.3}] Device {} selected:", elapsed, selected_device);
        devices[selected_device].print(start_time);
    }
}

// use DeviceStatus and timestamp to determine if the device is valid
// if the timestamp of EITHER temperature or humidity is not within the 
// DEVICE_TIMEOUT, assume there is an issue with the device and mark it as 
// invalid
fn set_device_valid(
    device: &mut Device,
) -> () {

    let temperature_duration = SystemTime::now().duration_since(device.temperature.1).unwrap();
    let humidity_duration = SystemTime::now().duration_since(device.humidity.1).unwrap();

    match device.status {
        DeviceStatus::OK => {
            if temperature_duration < DEVICE_TIMEOUT && humidity_duration < DEVICE_TIMEOUT {
                device.valid = true;
            } else {
                device.valid = false;
            }
        },
        DeviceStatus::DEAD => {
            device.valid = false;
        },
    }
}
