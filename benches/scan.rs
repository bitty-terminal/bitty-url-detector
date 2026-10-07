//! Throughput bench for `bitty-url-detector` (harness = false).
//!
//! Run with `cargo bench -p bitty-url-detector --bench scan`. Prints corpus
//! size, iteration count, total time, throughput (MiB/s), and detected URL
//! count. This is a local smoke measurement, not a cross-platform product
//! claim: report hardware, OS, build profile, and toolchain with any number.
//!
//! `#![forbid(unsafe_code)]`; `std` only.

#![forbid(unsafe_code)]

use std::hint::black_box;
use std::time::Instant;

use bitty_url_detector::detect_urls;

const ITERS: usize = 200;

fn corpus() -> String {
    let lines = [
        "Compiling bitty-url-detector v0.0.1 (see https://github.com/bitty-terminal/bitty-url-detector)",
        "Finished release profile [optimized] target(s) in 1.2s; docs at https://docs.rs/bitty-url-detector,",
        "plain log line without any link, just words and numbers 12345",
        "café 🎉 日本語 mixed line https://example.com/a?b=c#d and mailto:ops@example.com!",
        "clone git://git.example.com/repo.git (see https://en.wikipedia.org/wiki/Test_(a)).",
        "error: failed to fetch http://localhost:8080/health — retrying...",
    ];
    let mut blob = String::new();
    for _ in 0..200 {
        for line in lines {
            blob.push_str(line);
            blob.push('\n');
        }
    }
    blob
}

fn main() {
    let blob = corpus();
    // Warmup so the timed loop does not measure cold caches.
    let mut warm = 0;
    for _ in 0..5 {
        warm += detect_urls(black_box(&blob)).len();
    }
    black_box(warm);

    let start = Instant::now();
    let mut total_urls = 0;
    for _ in 0..ITERS {
        total_urls += detect_urls(black_box(&blob)).len();
    }
    let elapsed = start.elapsed();
    black_box(total_urls);

    let bytes = blob.len().saturating_mul(ITERS) as f64;
    let secs = elapsed.as_secs_f64().max(f64::EPSILON);
    let mib_s = bytes / secs / 1_048_576.0;
    println!("scan: {ITERS} iters, {} bytes total", blob.len() * ITERS);
    println!("scan: {elapsed:?} total, {mib_s:.2} MiB/s");
    println!("scan: {total_urls} URLs detected");
}
