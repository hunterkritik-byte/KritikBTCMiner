use sha2::{Digest, Sha256};
use std::env;
use std::time::Instant;

fn sha256d(data: &[u8]) -> [u8; 32] {
    let first = Sha256::digest(data);
    let second = Sha256::digest(first);
    let mut output = [0_u8; 32];
    output.copy_from_slice(&second);
    output
}

fn has_leading_zero_bits(hash: &[u8; 32], bits: u32) -> bool {
    if bits > 256 {
        return false;
    }

    let whole_bytes = (bits / 8) as usize;
    let remaining_bits = bits % 8;

    if hash[..whole_bytes].iter().any(|byte| *byte != 0) {
        return false;
    }

    remaining_bits == 0 || (hash[whole_bytes] >> (8 - remaining_bits)) == 0
}

fn search_demo_pow(message: &[u8], difficulty_bits: u32) -> (u64, [u8; 32], u64, u64) {
    let start = Instant::now();
    let mut input = Vec::with_capacity(message.len() + 8);
    input.extend_from_slice(message);

    for nonce in 0_u64..=u64::MAX {
        input.truncate(message.len());
        input.extend_from_slice(&nonce.to_le_bytes());

        let hash = sha256d(&input);
        if has_leading_zero_bits(&hash, difficulty_bits) {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            return (nonce, hash, elapsed_ms, nonce.saturating_add(1));
        }
    }

    unreachable!("u64 nonce space exhausted")
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let message = args.get(1).map(String::as_str).unwrap_or("hello from KritikBTCMiner");
    let difficulty_bits: u32 = args.get(2).map(|v| v.parse().unwrap_or(u32::MAX)).unwrap_or(18);

    if difficulty_bits > 32 {
        eprintln!("Demo difficulty must be between 0 and 32 leading zero bits.");
        eprintln!("Usage: cargo run -- \"message\" [difficulty_bits]");
        std::process::exit(2);
    }

    println!("KritikBTCMiner — educational proof-of-work demo");
    println!("Message: {message}");
    println!("Difficulty: {difficulty_bits} leading zero bits");
    println!("Note: this is not a Bitcoin block or a real network miner.");

    let (nonce, hash, elapsed_ms, hashes) = search_demo_pow(message.as_bytes(), difficulty_bits);
    let hash_hex: String = hash.iter().map(|byte| format!("{byte:02x}")).collect();
    let rate = if elapsed_ms == 0 { 0.0 } else { hashes as f64 * 1000.0 / elapsed_ms as f64 };

    println!("Found nonce: {nonce}");
    println!("SHA-256d: {hash_hex}");
    println!("Hashes tried: {hashes}");
    println!("Elapsed: {elapsed_ms} ms");
    if elapsed_ms > 0 {
        println!("Approx. search rate: {rate:.0} hashes/s");
    } else {
        println!("Approx. search rate: under 1 ms (too short to measure accurately)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_difficulty_always_matches() {
        assert!(has_leading_zero_bits(&[0xff; 32], 0));
    }

    #[test]
    fn checks_whole_zero_bytes_and_partial_bits() {
        let mut hash = [0_u8; 32];
        hash[1] = 0x80;
        assert!(has_leading_zero_bits(&hash, 8));
        assert!(!has_leading_zero_bits(&hash, 9));
    }

    #[test]
    fn rejects_difficulty_over_hash_width() {
        assert!(!has_leading_zero_bits(&[0; 32], 257));
    }

    #[test]
    fn sha256d_returns_32_bytes() {
        assert_eq!(sha256d(b"test").len(), 32);
    }
}
