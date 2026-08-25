//! Minimal reproduction of the backward-seek failure through the exact
//! production pipeline: Player + BufReader(HttpStreamSource).
//! Prints the FULL debug chain of any seek error.
//!
//! Run: cargo run --example backward_probe

use std::io::Read;
use std::time::{Duration, Instant};

use rustcast::player::HttpStreamSource;

const URL: &str = "https://traffic.megaphone.fm/FSI9558460816.mp3";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()?;

    let (_device, mixer) = {
        let handle = rodio::DeviceSinkBuilder::open_default_sink()?;
        let mixer = handle.mixer().clone();
        (handle, mixer)
    };
    let player = rodio::Player::connect_new(&mixer);

    let src = HttpStreamSource::open(&client, URL, 0)?;
    let content_length = src.content_length;
    let reader = std::io::BufReader::with_capacity(512 * 1024, src);
    let mut builder = rodio::decoder::DecoderBuilder::new()
        .with_data(reader)
        .with_seekable(true);
    if let Some(len) = content_length {
        builder = builder.with_byte_len(len);
    }
    let decoder = builder.build()?;
    player.append(decoder);
    println!("appended; waiting for playback…");

    let start = Instant::now();
    while player.get_pos() < Duration::from_secs_f64(2.5) {
        if start.elapsed() > Duration::from_secs(15) {
            return Err("playback never started".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // ---- Case 1: plain BACKWARD seek while playing -------------------
    println!("\n[case 1] try_seek(0.5s) while playing at {:.1}s", player.get_pos().as_secs_f64());
    match player.try_seek(Duration::from_secs_f64(0.5)) {
        Ok(()) => println!("  -> OK"),
        Err(e) => println!("  -> ERR\n{e:#?}"),
    }
    std::thread::sleep(Duration::from_millis(1200));
    println!("  pos now: {:.1}s", player.get_pos().as_secs_f64());

    // ---- Case 2: FORWARD seek ---------------------------------------
    println!("\n[case 2] try_seek(45s)");
    match player.try_seek(Duration::from_secs_f64(45.0)) {
        Ok(()) => println!("  -> OK"),
        Err(e) => println!("  -> ERR\n{e:#?}"),
    }
    std::thread::sleep(Duration::from_millis(1200));
    println!("  pos now: {:.1}s", player.get_pos().as_secs_f64());

    // ---- Case 3: BACKWARD seek after forward seek --------------------
    println!(
        "\n[case 3] try_seek(10s) after forward seek, playing at {:.1}s",
        player.get_pos().as_secs_f64()
    );
    match player.try_seek(Duration::from_secs_f64(10.0)) {
        Ok(()) => println!("  -> OK"),
        Err(e) => println!("  -> ERR\n{e:#?}"),
    }
    std::thread::sleep(Duration::from_millis(1500));
    println!("  pos now: {:.1}s", player.get_pos().as_secs_f64());

    // ---- Case 4: instrumented raw-source sanity ----------------------
    println!("\n[case 4] raw source re-open at byte(10s est) reads OK?");
    let mut s = HttpStreamSource::open(&client, URL, 160_000)?;
    let mut buf = vec![0u8; 4096];
    s.read_exact(&mut buf)?;
    println!("  read 4096 bytes @160000: OK");
    drop(s);

    Ok(())
}
