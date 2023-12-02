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
