# KritikBTCMiner

An open-source Rust project for learning and experimenting with Bitcoin proof-of-work.

> **Status:** Early development. The current program is an educational SHA-256d proof-of-work demo, **not** a production Bitcoin miner and does not mine real Bitcoin or connect to a mining pool yet.

## Goals

- Implement and test Bitcoin's double-SHA-256 primitive.
- Build a safe, measurable proof-of-work demonstration.
- Add proper Bitcoin block-header serialization and target validation.
- Explore Stratum pool integration and ASIC work submission in later milestones.

## Requirements

- Rust toolchain (stable)
- Cargo

On Termux, install the tools with:

```sh
pkg update
pkg install rust git
```

## Run

```sh
cargo test
cargo run -- "hello from KritikBTCMiner" 18
```

The second argument is the number of leading zero bits required in the demo hash (0–32). Higher values take longer on average. This demo searches a message-plus-nonce input; it is **not** a Bitcoin block header and a result is not valid for the Bitcoin network.

## Security notes

- Never paste or store wallet seed phrases or private keys in this project.
- Do not expect profitable Bitcoin mining on a phone CPU. Bitcoin mining generally requires specialized ASIC hardware.
- No wallet access, seed phrase scanning, or fund transfers are part of this project.

## Roadmap

- [x] Project scaffold and SHA-256d proof-of-work demo
- [ ] Add benchmarks and clearer runtime statistics
- [ ] Implement Bitcoin 80-byte block-header encoding and compact-target validation
- [ ] Add test vectors for header hashing and target comparison
- [ ] Research Stratum protocol and ASIC integration
- [ ] Document hardware, power, and pool requirements

## License

MIT. See [LICENSE](LICENSE).
