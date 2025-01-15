# Vortex

[![LICENSE](https://img.shields.io/github/license/dragonflyoss/vortex.svg?style=flat-square)](https://github.com/dragonflyoss/vortex/blob/main/LICENSE)
[![Discussions](https://img.shields.io/badge/discussions-on%20github-blue?style=flat-square)](https://github.com/dragonflyoss/vortex/discussions)

## Introduction

Vortex is a high-performance peer-to-peer (P2P) file transfer protocol implementation in Rust, designed as part of the Dragonfly project. It utilizes the TLV (Tag-Length-Value) format for efficient and flexible data transmission, making it ideal for large-scale file distribution scenarios.

### Key Features

- **Efficient TLV Protocol**: Optimized for fast serialization and deserialization of data packets
- **Large File Support**: Handles piece sizes up to 1 GiB and value fields up to 4 GiB
- **Memory Safety**: Built in Rust with zero-cost abstractions and memory safety guarantees
- **High Performance**: Designed for high-throughput file transfer scenarios
- **Extensible Design**: Reserved tags for future protocol extensions like compression and encryption

## Documentation

For details, please see [the specification](docs/README.md).

## Benchmarking

Vortex includes comprehensive benchmarks for packet creation, serialization, and deserialization:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark group
cargo bench "Vortex Packet Creation"
```

The benchmarks cover various packet sizes:

- Small packets (32 bytes)
- Medium packets (1 KB)
- Large packets (1 MB)
- Extra large packets (16 MB)

## LICENSE

Apache 2.0 License. Please see [LICENSE](LICENSE) for more information.
