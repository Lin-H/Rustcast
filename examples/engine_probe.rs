//! Headless probe: exercises the audio engine directly so seek/volume
//! behavior can be verified without the GUI.
//!
//! Run: cargo run --example engine_probe

use std::io::Cursor;
use std::time::{Duration, Instant};

use rustcast::player::{Command, PlayerHandle};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[1/6] fetching feed…");
    let xml = reqwest::blocking::get("https://feed.syntax.fm/")?.bytes()?;
    let raw = feed_rs::parser::parse(Cursor::new(&xml[..]))?;

    let (title, url, duration) = raw
        .entries
        .iter()
        .find_map(|e| {
            let content = e.media.iter().flat_map(|m| m.content.iter()).next()?;
            let url = content.url.as_ref()?.to_string();
            let dur = e
                .media
                .iter()
                .find_map(|m| m.duration)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            Some((
                e.title.as_ref().map(|t| t.content.clone()).unwrap_or_default(),
                url,
                dur,
            ))
        })
        .ok_or("no episode with audio enclosure found")?;

    println!("      episode: {title}");
    println!("      url:     {url}");
    println!("      rss duration: {duration:.0}s");

    let player = PlayerHandle::spawn();

    println!("[2/6] loading stream…");
    player.send(Command::Load {
        url,
        duration_secs: duration,
    });
    wait_for(&player, |s| !s.buffering && s.loaded_url.is_some(), 20_000);
    std::thread::sleep(Duration::from_millis(2500));
    report("after load", &player);

    println!("[3/6] set volume 0.25");
    player.send(Command::SetVolume(0.25));
    std::thread::sleep(Duration::from_millis(400));
    let snap = snap(&player);
    println!("      engine reports volume = {:.2} (expect 0.25)", snap.volume);

    let pos = snap.position.as_secs_f64();
    let fwd_target = pos + 60.0;
    println!("[4/6] FORWARD seek {pos:.1} -> {fwd_target:.1}");
    player.send(Command::Seek {
        seconds: fwd_target,
    });
    std::thread::sleep(Duration::from_millis(1800));
    report("after forward seek", &player);

    println!("[5/6] BACKWARD seek -> 12.0s");
    player.send(Command::Seek { seconds: 12.0 });
    std::thread::sleep(Duration::from_millis(1800));
    report("after backward seek", &player);

    println!("[6/6] BACKWARD seek again -> 3.0s");
    player.send(Command::Seek { seconds: 3.0 });
    std::thread::sleep(Duration::from_millis(1500));
    report("final", &player);

    Ok(())
}

fn snap(player: &PlayerHandle) -> rustcast::player::Snapshot {
    player.state.lock().unwrap().clone()
}

fn report(label: &str, player: &PlayerHandle) {
    let s = snap(player);
    println!(
        "      {label}: pos={:>7.1}s  playing={}  finished={}  err={:?}",
        s.position.as_secs_f64(),
        s.playing,
        s.finished,
        s.error
    );
}

fn wait_for(player: &PlayerHandle, pred: impl Fn(&rustcast::player::Snapshot) -> bool, ms: u64) {
    let deadline = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < deadline {
        if pred(&snap(player)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
