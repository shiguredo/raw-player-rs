use std::collections::VecDeque;
use std::sync::Mutex;

use crate::audio_format::AudioFormat;
use crate::audio_stream::AudioStream;
use crate::error::{Error, Result};
use crate::ffi;

struct AudioChunk {
    pts_us: i64,
    sample_rate: i32,
    channels: i32,
    format: AudioFormat,
    data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct AudioPlayerStats {
    pub audio_queue_size: usize,
    pub audio_buffer_ms: f64,
    pub chunks_played: i64,
    pub audio_clock_us: i64,
    pub sample_rate: i32,
    pub channels: i32,
    pub is_float: bool,
    pub total_samples_enqueued: i64,
    pub total_samples_played: i64,
    pub elapsed_time_ms: f64,
    pub audio_bitrate_kbps: f64,
}

struct AudioPlayerInner {
    stream: Option<AudioStream>,
    audio_queue: VecDeque<AudioChunk>,
    sample_rate: i32,
    channels: i32,
    is_float: bool,
    samples_written: i64,
    first_pts_us: i64,
    audio_started: bool,
    playing: bool,
    has_played: bool,
    volume: f32,
    chunks_played: i64,
    total_samples_enqueued: i64,
    play_start_time_ns: u64,
}

pub struct AudioPlayer {
    inner: Mutex<AudioPlayerInner>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(AudioPlayerInner {
                stream: None,
                audio_queue: VecDeque::new(),
                sample_rate: 0,
                channels: 0,
                is_float: false,
                samples_written: 0,
                first_pts_us: 0,
                audio_started: false,
                playing: false,
                has_played: false,
                volume: 1.0,
                chunks_played: 0,
                total_samples_enqueued: 0,
                play_start_time_ns: 0,
            }),
        }
    }

    /// PCM データをキューに追加する。
    pub fn enqueue_audio(
        &self,
        data: &[u8],
        pts_us: i64,
        sample_rate: i32,
        channels: i32,
        format: AudioFormat,
    ) -> Result<()> {
        if data.is_empty() {
            return Err(Error::invalid_argument("data is empty"));
        }
        if sample_rate <= 0 {
            return Err(Error::invalid_argument("sample_rate must be positive"));
        }
        if channels <= 0 {
            return Err(Error::invalid_argument("channels must be positive"));
        }

        let sample_size = format.sample_size();
        let frame_size = channels as usize * sample_size;
        if !data.len().is_multiple_of(frame_size) {
            return Err(Error::invalid_argument(
                "data size is not aligned to frame size",
            ));
        }

        let num_samples = data.len() as i64 / (channels as i64 * sample_size as i64);

        let mut inner = self.inner.lock().unwrap();

        if inner.has_played && !inner.playing {
            return Ok(());
        }

        inner.total_samples_enqueued += num_samples;
        inner.audio_queue.push_back(AudioChunk {
            pts_us,
            sample_rate,
            channels,
            format,
            data: data.to_vec(),
        });

        Ok(())
    }

    /// 再生を開始する。
    pub fn play(&self) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if !inner.playing {
            inner.playing = true;
            inner.has_played = true;
            if inner.play_start_time_ns == 0 {
                inner.play_start_time_ns = unsafe { ffi::SDL_GetTicksNS() };
            }
            Self::process_audio_queue(&mut inner)?;
            if let Some(ref mut stream) = inner.stream {
                stream.resume()?;
            }
        }
        Ok(())
    }

    /// 再生を一時停止する。
    pub fn pause(&self) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if inner.playing {
            inner.playing = false;
            if let Some(ref mut stream) = inner.stream {
                stream.pause()?;
            }
        }
        Ok(())
    }

    /// 再生を停止してキューをクリアする。
    pub fn stop(&self) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.playing = false;
        inner.audio_queue.clear();
        if let Some(ref mut stream) = inner.stream {
            stream.pause()?;
            stream.clear()?;
        }
        inner.samples_written = 0;
        inner.audio_started = false;
        inner.total_samples_enqueued = 0;
        inner.play_start_time_ns = 0;
        inner.chunks_played = 0;
        Ok(())
    }

    pub fn is_playing(&self) -> bool {
        self.inner.lock().unwrap().playing
    }

    pub fn volume(&self) -> f32 {
        self.inner.lock().unwrap().volume
    }

    pub fn set_volume(&self, volume: f32) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        let clamped = volume.clamp(0.0, 1.0);
        inner.volume = clamped;
        if let Some(ref mut stream) = inner.stream {
            stream.set_gain(clamped)?;
        }
        Ok(())
    }

    /// 現在の音声再生位置をマイクロ秒で返す。
    pub fn audio_clock_us(&self) -> i64 {
        let inner = self.inner.lock().unwrap();
        Self::get_audio_clock_us(&inner)
    }

    /// 音声再生が開始されたかを返す。
    pub fn is_started(&self) -> bool {
        self.inner.lock().unwrap().audio_started
    }

    /// キューに溜まったチャンクをストリームにフラッシュする (再生中のみ)。
    pub fn process(&self) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if inner.playing {
            Self::process_audio_queue(&mut inner)?;
        }
        Ok(())
    }

    /// 音声キューのバッファ量をミリ秒で返す。
    pub fn audio_queue_ms(&self) -> f64 {
        let inner = self.inner.lock().unwrap();
        if inner.sample_rate > 0 {
            let sample_size: i64 = if inner.is_float { 4 } else { 2 };
            let bytes_per_frame = inner.channels as i64 * sample_size;
            let queued_bytes: i64 = inner.audio_queue.iter().map(|c| c.data.len() as i64).sum();
            let stream_queued = inner
                .stream
                .as_ref()
                .map(|s| s.queued_bytes() as i64)
                .unwrap_or(0);
            let total = queued_bytes + stream_queued;
            if bytes_per_frame > 0 {
                let samples = total / bytes_per_frame;
                samples as f64 * 1000.0 / inner.sample_rate as f64
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    pub fn stats(&self) -> AudioPlayerStats {
        let inner = self.inner.lock().unwrap();

        let clock_us = Self::get_audio_clock_us(&inner);

        let buffer_ms = if inner.sample_rate > 0 {
            let sample_size = if inner.is_float { 4 } else { 2 };
            let bytes_per_frame = inner.channels as i64 * sample_size;
            let total_queued_bytes: i64 =
                inner.audio_queue.iter().map(|c| c.data.len() as i64).sum();
            let stream_queued = inner
                .stream
                .as_ref()
                .map(|s| s.queued_bytes() as i64)
                .unwrap_or(0);
            let total_bytes = total_queued_bytes + stream_queued;
            if bytes_per_frame > 0 {
                let samples = total_bytes / bytes_per_frame;
                samples as f64 * 1000.0 / inner.sample_rate as f64
            } else {
                0.0
            }
        } else {
            0.0
        };

        let elapsed_ms = if inner.has_played {
            let now = unsafe { ffi::SDL_GetTicksNS() };
            (now - inner.play_start_time_ns) as f64 / 1_000_000.0
        } else {
            0.0
        };

        let sample_size = if inner.is_float { 4 } else { 2 };
        let bitrate_kbps = if inner.sample_rate > 0 {
            inner.sample_rate as f64 * inner.channels as f64 * sample_size as f64 * 8.0 / 1000.0
        } else {
            0.0
        };

        let queued_samples = inner
            .stream
            .as_ref()
            .map(|s| {
                let bytes = s.queued_bytes() as i64;
                let bytes_per_frame = inner.channels as i64 * sample_size;
                if bytes_per_frame > 0 {
                    bytes / bytes_per_frame
                } else {
                    0
                }
            })
            .unwrap_or(0);
        let played_samples = (inner.samples_written - queued_samples).max(0);

        AudioPlayerStats {
            audio_queue_size: inner.audio_queue.len(),
            audio_buffer_ms: buffer_ms,
            chunks_played: inner.chunks_played,
            audio_clock_us: clock_us,
            sample_rate: inner.sample_rate,
            channels: inner.channels,
            is_float: inner.is_float,
            total_samples_enqueued: inner.total_samples_enqueued,
            total_samples_played: played_samples,
            elapsed_time_ms: elapsed_ms,
            audio_bitrate_kbps: bitrate_kbps,
        }
    }

    fn get_audio_clock_us(inner: &AudioPlayerInner) -> i64 {
        if !inner.audio_started || inner.stream.is_none() {
            return 0;
        }

        let sample_size: i64 = if inner.is_float { 4 } else { 2 };
        let bytes_per_frame = inner.channels as i64 * sample_size;

        let queued_bytes = inner
            .stream
            .as_ref()
            .map(|s| s.queued_bytes() as i64)
            .unwrap_or(0);

        let queued_samples = if bytes_per_frame > 0 {
            queued_bytes / bytes_per_frame
        } else {
            0
        };

        let played_samples = (inner.samples_written - queued_samples).max(0);

        if inner.sample_rate > 0 {
            let played_us = played_samples * 1_000_000 / inner.sample_rate as i64;
            inner.first_pts_us + played_us
        } else {
            0
        }
    }

    fn process_audio_queue(inner: &mut AudioPlayerInner) -> Result<()> {
        while let Some(chunk) = inner.audio_queue.pop_front() {
            let format_changed = inner.stream.is_some()
                && (chunk.sample_rate != inner.sample_rate
                    || chunk.channels != inner.channels
                    || chunk.format.is_float() != inner.is_float);

            if inner.stream.is_none() || format_changed {
                inner.stream = None;
                let mut stream =
                    AudioStream::open(chunk.sample_rate, chunk.channels, chunk.format)?;
                stream.set_gain(inner.volume)?;
                if inner.playing {
                    stream.resume()?;
                }
                inner.stream = Some(stream);
                inner.sample_rate = chunk.sample_rate;
                inner.channels = chunk.channels;
                inner.is_float = chunk.format.is_float();
                inner.samples_written = 0;
            }

            if !inner.audio_started {
                inner.first_pts_us = chunk.pts_us;
                inner.audio_started = true;
            }

            let sample_size = if inner.is_float { 4 } else { 2 };
            let bytes_per_frame = inner.channels as i64 * sample_size;
            let num_samples = if bytes_per_frame > 0 {
                chunk.data.len() as i64 / bytes_per_frame
            } else {
                0
            };

            if let Some(ref mut stream) = inner.stream {
                stream.put_data(&chunk.data)?;
            }

            inner.samples_written += num_samples;
            inner.chunks_played += 1;
        }
        Ok(())
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}
