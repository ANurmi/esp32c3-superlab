## Commands

- Set current "reference" date and time:
`set(id = 1, Message::A(UtcDateTime), DevID)`

- Configure LED Blinker to be off now
`set(id = 2, Message::B(<doesn't matter>), DevID)`

- Turn LED Blinker on right now, for a set duration, at a set frequency
`set(id = 3, Message::C(duration_secs, frequency_hz), DevID)`

- Turn LED Blinker on at a set time, for a set duration, at a set frequency
`set(id = 4, Message::D(UtcDateTime, duration_secs, frequency_hz), DevID)`

- Turn RGB LED on/off
`set(id = 5, Message::B(0/1), DevID)`