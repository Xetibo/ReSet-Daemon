# ReSet-Daemon
This is the standalone daemon for [ReSet](https://github.com/Xetibo/ReSet) written in rust.\
It provides all functionality accessible within ReSet and can also be accessed by any other application via DBus.

## features

- Bluetooth via bluez
- Audio via PulseAudio
- Wi-Fi via NetworkManager
## Usage
For Usage, consult the [documentation](https://docs.rs/reset_daemon/0.6.9/reset_daemon/).

When delevoping an appliation that interacts with ReSet-Daemon, consider using the [ReSet-Lib](https://github.com/Xetibo/ReSet-Lib) which provides preconfigured datastructures. The API is also available in the documentation linked above.

## Installation
The daemon currently only offers installation via crates.io or via manual compilation:

```
cargo install reset_daemon
```
## Roadmap

This application was developed as a semester project for the Eastern Switzerland University of Applied Sciences.
With potential advancements as a next project, due to this, no major development will happen until February 2024.
However, there is still a roadmap for this application.

- Plugin System
- Better Error handling
