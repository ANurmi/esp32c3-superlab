# esp32c3-wifi-mqtt-demo

A repo for developing/experimenting  wifi-mqtt exercise for Reliable Embedded System course in Tampere University

The majority of this project uses code bases from the following projects:

1. Esprissif std-training (host-client):
    <https://github.com/esp-rs/std-training.git>
2. Esp RTIC from: <https://github.com/perlindgren/esp32c3-test.git>
3. No-std MQTT from: <https://github.com/JurajSadel/esp32c3-no-std-async-mqtt-demo.git>

## System architecture

![System architecture](./figures/sys.jpg?raw=true)

## How to run

- Run host-client (on PC):
  - `cargo run`

- Run esp-no-std-mqtt on ESP32C3 (in one or more)
  - `cargo run --release`

# Exercise 3 Notes

## Device

Device code can be found within the esp32c3-no-std-mqtt directory. The demo has been designed to run with up to 3 devices. The UUID for each device is statically defined using the UUID constant and can be selected by uncommenting the appropriate line starting on line 57 of main.rs. The UUID of the device also corresponds to the proposed position of the device on the drilling robot in accordance with the following table:

| Position   | Device ID | UUID                                   |
|------------|-----------|----------------------------------------|
| Rear       | 0         | "16e337a0-935d-4f32-bf3c-6ded006cesp0" |
| Top        | 1         | "16e337a0-935d-4f32-bf3c-6ded006cesp1" |
| Front Left | 2         | "16e337a0-935d-4f32-bf3c-6ded006cesp2" |

Each device attempts to connect to a network and then publish the temperature and humidity data from the SHTC3 sensor on the board at a frequency of 2Hz.

## Host

The host code attempts to subscribe to the temperature and density of each of the 3 sensors. If no new device data is published for a statically configurable period of time (defined by DEVICE_TIMEOUT), the host reports that there are no valid devices.

A device is considered valid if it has published BOTH temperature AND humidity data, with neither values being older than the time period defined by DEVICE_TIMEOUT. Otherwise, the device is considered to be invalid and its data will not be selected for presentation. If more than one device is considered to be valid, the data selected for presentation is chosen based on the reliability of the device. This means that the priority for selection is as follows:

```
 Highest Priority
|-----------------|
  Device 1 (Rear)
        |
        v
  Device 2 (Top)
        |
        v
  Device 3 (Front)
|-----------------|
 Lowest Priority
 ```
 
 If the data from the currently selected (highest priority) device becomes invalid at any point, the selected device will be switched to the next highest priority device with valid data. If the higher priority device data becomes valid again, the host will switch the selected device back, therefore always guaranteeing that the highest priority valid data is displayed.