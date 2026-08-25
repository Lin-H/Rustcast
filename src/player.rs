use std::io::{Read, Seek, SeekFrom};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug)]
pub enum Command {
    Load {
        url: String,
        duration_secs: f64,
    },
    TogglePlay,
    Play,
    Pause,
    Seek {
        seconds: f64,
    },
    SetVolume(f32),
    Stop,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snapshot {
    pub loaded_url: Option<String>,
    pub playing: bool,
    pub finished: bool,
    pub buffering: bool,
    pub position: Duration,
    pub duration: Option<Duration>,
    pub volume: f32,
    pub error: Option<String>,
}

/// A `Read + Seek` adapter over a streaming HTTP response.
///
/// Forward seeks are emulated by reading-and-discarding; backward seeks
/// issue a fresh request with a `Range` header. This gives symphonia's
/// decoder everything it needs to support time-based seeks mid-stream.
pub struct HttpStreamSource {
    client: reqwest::blocking::Client,
    url: String,
    response: reqwest::blocking::Response,
    offset: u64,
    pub content_length: Option<u64>,
}

impl HttpStreamSource {
    pub fn open(
        client: &reqwest::blocking::Client,
        url: &str,
        start: u64,
    ) -> reqwest::Result<Self> {
        let response = Self::open_raw(client, url, start)?;
        let content_length = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .map(|n| n + start);
        Ok(Self {
            client: client.clone(),
            url: url.to_owned(),
            response,
            offset: start,
            content_length,
        })
    }

    fn open_raw(
        client: &reqwest::blocking::Client,
        url: &str,
        start: u64,
    ) -> reqwest::Result<reqwest::blocking::Response> {
        let mut req = client.get(url);
        if start > 0 {
            req = req.header(reqwest::header::RANGE, format!("bytes={start}-"));
        }
        req.send()?.error_for_status()
    }
}

impl Read for HttpStreamSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.response.read(buf)
    }
}

impl Seek for HttpStreamSource {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let target: i64 = match pos {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(delta) => self.offset as i64 + delta,
            SeekFrom::End(delta) => {
                let len = self.content_length.ok_or_else(|| {
                    std::io::Error::other("stream length unknown; cannot seek from end")
                })?;
                len as i64 + delta
            }
        };
        if target < 0 {
            return Err(std::io::Error::other("seek before start of stream"));
        }
        let target = target as u64;

        if target >= self.offset {
            let mut remaining = target - self.offset;
            let mut scratch = [0u8; 16 * 1024];
            while remaining > 0 {
                let want = remaining.min(scratch.len() as u64) as usize;
                let n = self.response.read(&mut scratch[..want])?;
                if n == 0 {
                    break;
                }
                self.offset += n as u64;
                remaining -= n as u64;
            }
        } else {
            let resp =
                Self::open_raw(&self.client, &self.url, target).map_err(std::io::Error::other)?;
            // A server that ignores Range replies 200 with the FULL body,
            // which would silently desync our offset tracking.
            if target > 0 && resp.status() != reqwest::StatusCode::PARTIAL_CONTENT {
                return Err(std::io::Error::other(format!(
                    "server does not support range requests (HTTP {})",
                    resp.status()
                )));
            }
            self.response = resp;
            self.offset = target;
        }
        Ok(self.offset)
    }
}

struct Engine {
    cmd_rx: Receiver<Command>,
    shared: Arc<Mutex<Snapshot>>,
    player: rodio::Player,
    _device: rodio::MixerDeviceSink,
    client: reqwest::blocking::Client,
    current_url: Option<String>,
    paused: bool,
    /// Flips true once the queue has been observed non-empty, so that
    /// natural end-of-stream (`empty()` going back to true) can be detected.
    started: bool,
    volume: f32,
}

impl Engine {
    fn run(mut self) {
        loop {
            match self.cmd_rx.recv_timeout(Duration::from_millis(80)) {
                Ok(cmd) => {
                    if matches!(cmd, Command::Stop) {
                        self.reset_shared();
                        continue;
                    }
                    self.handle(cmd);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
            self.publish_status();
        }
    }

    fn handle(&mut self, cmd: Command) {
        match cmd {
            Command::Load {
                url,
                duration_secs,
            } => self.load(&url, duration_secs),
            Command::TogglePlay => {
                if self.shared.lock().unwrap().loaded_url.is_none() {
                    return;
                }
                if self.player.is_paused() {
                    self.player.play();
                    self.paused = false;
                } else {
                    self.player.pause();
                    self.paused = true;
                }
            }
            Command::Play => {
                self.player.play();
                self.paused = false;
            }
            Command::Pause => {
                self.player.pause();
                self.paused = true;
            }
            Command::Seek { seconds } => {
                if self.current_url.is_none() {
                    return;
                }
                if let Err(e) = self.player.try_seek(Duration::from_secs_f64(seconds.max(0.0))) {
                    let mut st = self.shared.lock().unwrap();
                    st.error = Some(format!("跳转失败: {e}"));
                } else {
                    let mut st = self.shared.lock().unwrap();
                    st.error = None;
                    st.finished = false;
                }
            }
            Command::SetVolume(v) => {
                self.volume = v.clamp(0.0, 1.0);
                self.player.set_volume(self.volume);
            }
            Command::Stop => unreachable!(),
        }
    }

    fn load(&mut self, url: &str, duration_secs: f64) {
        {
            let mut st = self.shared.lock().unwrap();
            st.buffering = true;
            st.error = None;
            st.finished = false;
            st.playing = false;
            st.position = Duration::ZERO;
            st.duration = if duration_secs > 0.0 {
                Some(Duration::from_secs_f64(duration_secs))
            } else {
                None
            };
            st.loaded_url = Some(url.to_owned());
        }
        self.suspend_playback();
        self.current_url = Some(url.to_owned());

        let opened = HttpStreamSource::open(&self.client, url, 0).map_err(|e| e.to_string());
        let src = match opened {
            Ok(src) => src,
            Err(e) => {
                let mut st = self.shared.lock().unwrap();
                st.buffering = false;
                st.error = Some(format!("无法连接音频流: {e}"));
                return;
            }
        };

        let content_length = src.content_length;
        let reader = std::io::BufReader::with_capacity(512 * 1024, src);
        // Declaring byte_len + seekable is what allows symphonia's MP3
        // demuxer to perform BACKWARD seeks; without it only forward
        // seeking works (SeekErrorKind::ForwardOnly).
        let mut builder = rodio::decoder::DecoderBuilder::new()
            .with_data(reader)
            .with_seekable(true);
        if let Some(len) = content_length {
            builder = builder.with_byte_len(len);
        }
        match builder.build() {
            Ok(decoder) => {
                self.player.append(decoder);
                self.player.set_volume(self.volume);
                self.player.play();
                self.paused = false;
                self.started = false;
                let mut st = self.shared.lock().unwrap();
                st.buffering = false;
                st.playing = true;
                st.position = Duration::ZERO;
            }
            Err(e) => {
                let mut st = self.shared.lock().unwrap();
                st.buffering = false;
                st.error = Some(format!("音频解码失败: {e}"));
            }
        }
    }

    /// Empty the queue and leave the player paused, ready for a new source.
    fn suspend_playback(&mut self) {
        self.player.stop();
        self.player.pause();
        self.paused = true;
        self.started = false;
    }

    fn publish_status(&mut self) {
        let mut st = self.shared.lock().unwrap();
        if st.loaded_url.is_none() || st.buffering {
            return;
        }
        if !self.started && !self.player.empty() {
            self.started = true;
        }
        let finished = self.started && self.player.empty();

        st.finished = finished;
        st.playing = !self.paused && !finished;
        st.position = self.player.get_pos();
        st.volume = self.volume;
    }

    fn reset_shared(&mut self) {
        self.suspend_playback();
        self.current_url = None;
        let mut st = self.shared.lock().unwrap();
        *st = Snapshot {
            volume: self.volume,
            ..Snapshot::default()
        };
    }
}

/// Thread-safe front-end of the playback engine.
#[derive(Clone)]
pub struct PlayerHandle {
    tx: Sender<Command>,
    pub state: Arc<Mutex<Snapshot>>,
}

impl PlayerHandle {
    pub fn spawn() -> Self {
        let (tx, rx) = channel();
        let shared = Arc::new(Mutex::new(Snapshot::default()));

        let engine_state = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("audio-engine".into())
            .spawn(move || {
                let report = Arc::clone(&engine_state);
                match build_engine(rx, engine_state) {
                    Ok(engine) => engine.run(),
                    Err(e) => {
                        if let Ok(mut st) = report.lock() {
                            st.error = Some(format!("音频设备初始化失败: {e}"));
                        }
                    }
                }
            })
            .expect("failed to spawn audio thread");

        Self { tx, state: shared }
    }

    pub fn send(&self, cmd: Command) {
        let _ = self.tx.send(cmd);
    }
}

fn build_engine(
    cmd_rx: Receiver<Command>,
    shared: Arc<Mutex<Snapshot>>,
) -> Result<Engine, String> {
    let device = rodio::DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string())?;
    let player = rodio::Player::connect_new(device.mixer());
    player.pause();
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    Ok(Engine {
        cmd_rx,
        shared,
        player,
        _device: device,
        client,
        current_url: None,
        paused: true,
        started: false,
        volume: 1.0,
    })
}
