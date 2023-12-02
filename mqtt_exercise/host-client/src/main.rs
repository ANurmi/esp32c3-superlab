use mqtt_topics::{temperature_data_topic, Esp};
use rumqttc::{Client, MqttOptions, Packet, QoS};
use std::error::Error;

const UUID: &'static str = get_uuid::uuid();
const UUID_ESP0: &'static str = "16e337a0-935d-4f32-bf3c-6ded006cesp0";
const UUID_ESP1: &'static str = "16e337a0-935d-4f32-bf3c-6ded006cesp1";
const UUID_ESP2: &'static str = "16e337a0-935d-4f32-bf3c-6ded006cesp2";


fn main() -> Result<(), Box<dyn Error>> {
    let client_id = UUID;
    let mqtt_host = "test.mosquitto.org";
    dbg!(UUID);

    let mqttoptions = MqttOptions::new(client_id, mqtt_host, 1883);

    let (mut client, mut connection) = Client::new(mqttoptions, 100);
    client.subscribe(
        temperature_data_topic(UUID_ESP0, Esp::EspTarget1).as_str(),
        QoS::AtMostOnce,
    )?;
    client.subscribe(
        temperature_data_topic(UUID_ESP1, Esp::EspTarget1).as_str(),
        QoS::AtMostOnce,
    )?;
    client.subscribe(
        temperature_data_topic(UUID_ESP2, Esp::EspTarget1).as_str(),
        QoS::AtMostOnce,
    )?;

    // Iterate to poll the eventloop for connection progress
    for (_, notification) in connection.iter().enumerate() {
        // if you want to see *everything*, uncomment:
        // println!("Notification = {:#?}", notification);
        if let Ok(rumqttc::Event::Incoming(Packet::Publish(publish_data))) = notification {
            
            if publish_data.topic == temperature_data_topic(UUID_ESP0, Esp::EspTarget1).as_str() {
                
                let data: &[u8] = &publish_data.payload;

                let presentation_data : String = temp_arr_to_str(data);
                
                println!("ESP0_temperature = {:?}", presentation_data);

                let data: Result<[u8; 4], _> = data.try_into();

                if let Ok(data) = data {
                    println!("{:?}", data)
                }
            }

            if publish_data.topic == temperature_data_topic(UUID_ESP1, Esp::EspTarget1).as_str() {
                
                let data: &[u8] = &publish_data.payload;
                
                let presentation_data : String = temp_arr_to_str(data);
                println!("ESP1_temperature = {:?}", presentation_data);
                
                let data: Result<[u8; 4], _> = data.try_into();

                if let Ok(data) = data {
                    println!("{:?}", data)
                }
            }

            if publish_data.topic == temperature_data_topic(UUID_ESP2, Esp::EspTarget1).as_str() {
                
                let data: &[u8] = &publish_data.payload;
                
                let presentation_data : String = temp_arr_to_str(data);
                println!("ESP2_temperature = {:?}", presentation_data);
                
                let data: Result<[u8; 4], _> = data.try_into();

                if let Ok(data) = data {
                    println!("{:?}", data)
                }
            }
        }
    }
    Ok(())
}

fn temp_arr_to_str(temp_arr: &[u8]) -> String {
    let mut temp_str = String::new();

    for element in temp_arr {
        temp_str.push(*element as char);
    }

    temp_str
}
