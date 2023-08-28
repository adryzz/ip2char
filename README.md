# ip2char
**Tunnel IP traffic through serial ports and other transports**

## Transports supported
- [x] Serial ports (char devices)
- [x] TCP (useful for quick debugging)

## IPv6 support
IPv6 support is planned but a dependency, [`packet`](https://github.com/meh/rust-packet) doesn't yet allow reading the destination from IPv6 packets.

`ip2char` does packet filtering, to ensure that the tiny bandwidth of the transport isn't taken up by packets that would get filtered by the kernel at the other side of the tunnel.

You can disable it in the configuration file, and allow IPv6 and broadcast packets to get routed, but the transport will become slower as a result.

# Features
- [x] Error correction
- [x] Compression
- [ ] Encryption

# Configuration
Very inspired from Wireguard

```toml
[interface]
address = "10.1.0.1/24"
name = "ip2char0"

[[peer-char]]
path = "/dev/ttyACM0"
allowedips = ["10.1.0.2/32"]
# default serial speed is 115200 bps

[[peer-char]]
path = "/dev/ttyACM1"
allowedips = ["10.1.0.3/32", "10.1.0.4/32"]
speed = 230400
```