# mizu-vizu

A terminal audio visualizer written in Rust.

## Limitations

* The audio device is currently hardcoded for the PulseAudio backend,
so it may or may not work for your device.
* **The application panics for devices with other than 2 audio channels!**

## Support

* PulseAudio on Linux
* WinAPI + IAudioCaptureClient on Windows

## Building

WARNING: *Very slow in debug mode.*

```shell
cargo build
```

## Running

Run the program in release mode via Cargo:

```shell
cargo run --release
```
