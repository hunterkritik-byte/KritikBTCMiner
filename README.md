# KritikBTCMiner

KritikBTCMiner is an experimental Rust Stratum V1 CPU mining client for Bitcoin-compatible SHA-256d pools.

It connects to a pool, subscribes and authorizes, parses jobs, constructs an 80-byte block header, hashes candidate headers with double SHA-256, compares hashes with the pool share target, and submits candidate shares.

## Important limitations

- This is an early experimental implementation, not audited or production-grade. Validate protocol byte order and share handling against a trusted pool before relying on it.
- It is not expected to earn meaningful BTC on an Android phone. Bitcoin mining is dominated by specialized ASIC hardware.
- It currently supports unencrypted Stratum V1 TCP only. TLS, reconnect/backoff, complete extranonce/job lifecycle handling, version rolling, time rolling, and robust multi-worker management remain to be implemented.
- A pool share is not a Bitcoin block. Payouts depend on pool policy, accepted shares, fees, and payout thresholds.
- Never put wallet seed phrases or private keys into this project. Never commit pool credentials.

## Termux setup

```sh
pkg update
pkg install rust git
git clone https://github.com/hunterkritik-byte/KritikBTCMiner.git
cd KritikBTCMiner
cargo test
cargo build --release
```

## Configure and run

Use the host, port, username/worker and password supplied by a pool you trust. The hostname below is only a placeholder.

```sh
export BTC_POOL_HOST="your-pool-host.example"
export BTC_POOL_PORT="3333"
export BTC_POOL_USER="your-pool-username.worker1"
export BTC_POOL_PASSWORD="x"
cargo run --release
```

Do not use these placeholder values as a real endpoint. Many pools use x as the worker password, but follow your provider's instructions. This implementation uses plain TCP; do not send credentials over untrusted networks.

## Current implementation

- Stratum V1 TCP connection
- Subscribe and authorize requests
- Pool difficulty and job notifications
- Coinbase and merkle-root construction
- 80-byte block header assembly
- SHA-256d nonce search
- Share-target comparison
- Share submission and accepted/rejected response logging
- Basic unit tests

## Remaining production hardening

- [ ] Validate header/merkle byte order with published vectors and a local Stratum test server
- [ ] Add Stratum TLS and safe reconnect/backoff
- [ ] Handle extranonce changes and job cancellation edge cases correctly
- [ ] Add time rolling, version rolling, multi-worker support and accurate hashrate reporting
- [ ] Add end-to-end pool integration tests
- [ ] Benchmark on supported hardware; use ASICs for realistic Bitcoin mining

## License

MIT. See LICENSE.
