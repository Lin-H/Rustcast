//! Isolates the backward-seek failure.
//!
//! Stage A verifies that ranged HTTP reads return byte-exact data.
//! Stage B drives a rodio Decoder directly (no Player) and prints the
//! FULL debug chain of the seek error.
//!
//! Run: cargo run --example seek_probe

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use rodio::source::Source;
use rustcast::player::HttpStreamSource;

const URL: &str = "https://traffic.megaphone.fm/FSI9558460816.mp3";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()?;

    println!("=== Stage A: ranged read byte-correctness ===");

    let mut a = HttpStreamSource::open(&client, URL, 1_000_000)?;
    let mut buf_a = vec![0u8; 4096];
    a.read_exact(&mut buf_a)?;

    let mut b = HttpStreamSource::open(&client, URL, 1_000_000 + 2048)?;
    let mut buf_b = vec![0u8; 2048];
    b.read_exact(&mut buf_b)?;

    println!(
        "overlap check (open@1000000[2048..4096] vs open@1002048): {}",
        if buf_a[2048..] == buf_b { "PASS" } else { "FAIL" }
    );

    let mut s = HttpStreamSource::open(&client, URL, 500_000)?;
    let mut scratch = vec![0u8; 1024];
    s.read_exact(&mut scratch)?;
    let before = s.seek(SeekFrom::Start(300_000))?;
    println!("seek(Start(300000)) returned offset = {before}");
    let mut via_seek = vec![0u8; 1024];
    s.read_exact(&mut via_seek)?;

    let mut direct = HttpStreamSource::open(&client, URL, 300_000)?;
    let mut via_direct = vec![0u8; 1024];
    direct.read_exact(&mut via_direct)?;

    println!(
        "backward-seek data check (seek-path vs fresh-open): {}",
        if via_seek == via_direct { "PASS" } else { "FAIL" }
    );

    println!("=== Stage B: decoder-level backward seek ===");
    let src = HttpStreamSource::open(&client, URL, 0)?;
    let reader = std::io::BufReader::with_capacity(512 * 1024, src);
    let mut decoder = rodio::Decoder::try_from(reader)?;
    println!("decoder created");

    // consume ~1s of audio (assume ≤48kHz stereo) to simulate playback state
    let target_samples = 48_000 * 2;
    let mut consumed = 0usize;
    while consumed < target_samples {
        match decoder.next() {
            Some(_) => consumed += 1,
            None => {
                println!("stream ended early at {consumed} samples");
                break;
            }
        }
    }
    println!(
        "consumed {consumed} samples (~{}ms @48k stereo)",
        consumed / 96
    );
    drop(scratch);

    println!("attempting Source::try_seek(12s)…");
    match decoder.try_seek(Duration::from_secs(12)) {
        Ok(()) => println!("BACKWARD SEEK OK"),
        Err(e) => {
            println!("SEEK FAILED");
            println!("display: {e}");
            println!("debug chain:\n{e:?}");
        }
    }

    Ok(())
}
