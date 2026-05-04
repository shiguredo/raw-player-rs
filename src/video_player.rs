use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::audio_format::AudioFormat;
use crate::audio_player::AudioPlayer;
use crate::error::{Error, Result};
use crate::ffi;
use crate::pixel_buffer::PixelBufferRef;
use crate::renderer::Renderer;
use crate::texture::Texture;
use crate::video_format::VideoFormat;
use crate::video_frame::{FrameData, VideoFrame};
use crate::window::Window;
use crate::{Event, KEYCODE_S, poll_event};

/// pitch 計算 (`width * 4`) で i32 オーバーフローしない最大 width。
const MAX_DIMENSION: i32 = i32::MAX / 4;

/// 入力検証: I420 フレームデータのサイズを検証する。
pub fn validate_i420(y: &[u8], u: &[u8], v: &[u8], width: i32, height: i32) -> Result<()> {
    if width <= 0 || height <= 0 {
        return Err(Error::invalid_argument("width and height must be positive"));
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(Error::invalid_argument(format!(
            "dimensions too large: {width}x{height} (max {MAX_DIMENSION})"
        )));
    }
    if width % 2 != 0 || height % 2 != 0 {
        return Err(Error::invalid_argument(
            "I420 requires even width and height",
        ));
    }
    let w = width as usize;
    let h = height as usize;
    let expected_y = w * h;
    let expected_uv = (w / 2) * (h / 2);
    if y.len() != expected_y {
        return Err(Error::invalid_argument(format!(
            "Y plane size mismatch: expected {expected_y}, got {}",
            y.len()
        )));
    }
    if u.len() != expected_uv {
        return Err(Error::invalid_argument(format!(
            "U plane size mismatch: expected {expected_uv}, got {}",
            u.len()
        )));
    }
    if v.len() != expected_uv {
        return Err(Error::invalid_argument(format!(
            "V plane size mismatch: expected {expected_uv}, got {}",
            v.len()
        )));
    }
    Ok(())
}

/// 入力検証: NV12 フレームデータのサイズを検証する。
pub fn validate_nv12(y: &[u8], uv: &[u8], width: i32, height: i32) -> Result<()> {
    if width <= 0 || height <= 0 {
        return Err(Error::invalid_argument("width and height must be positive"));
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(Error::invalid_argument(format!(
            "dimensions too large: {width}x{height} (max {MAX_DIMENSION})"
        )));
    }
    if width % 2 != 0 || height % 2 != 0 {
        return Err(Error::invalid_argument(
            "NV12 requires even width and height",
        ));
    }
    let w = width as usize;
    let h = height as usize;
    let expected_y = w * h;
    let expected_uv = w * (h / 2);
    if y.len() != expected_y {
        return Err(Error::invalid_argument(format!(
            "Y plane size mismatch: expected {expected_y}, got {}",
            y.len()
        )));
    }
    if uv.len() != expected_uv {
        return Err(Error::invalid_argument(format!(
            "UV plane size mismatch: expected {expected_uv}, got {}",
            uv.len()
        )));
    }
    Ok(())
}

/// 入力検証: stride 付き I420 フレームデータのサイズを検証する。
pub fn validate_i420_strided(
    y: &[u8],
    u: &[u8],
    v: &[u8],
    width: i32,
    height: i32,
    y_pitch: i32,
    uv_pitch: i32,
) -> Result<()> {
    if width <= 0 || height <= 0 {
        return Err(Error::invalid_argument("width and height must be positive"));
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(Error::invalid_argument(format!(
            "dimensions too large: {width}x{height} (max {MAX_DIMENSION})"
        )));
    }
    if width % 2 != 0 || height % 2 != 0 {
        return Err(Error::invalid_argument(
            "I420 requires even width and height",
        ));
    }
    if y_pitch < width {
        return Err(Error::invalid_argument(format!(
            "y_pitch ({y_pitch}) must be >= width ({width})"
        )));
    }
    if uv_pitch < width / 2 {
        return Err(Error::invalid_argument(format!(
            "uv_pitch ({uv_pitch}) must be >= width/2 ({})",
            width / 2
        )));
    }
    let h = height as usize;
    let expected_y = y_pitch as usize * h;
    let expected_uv = uv_pitch as usize * (h / 2);
    if y.len() != expected_y {
        return Err(Error::invalid_argument(format!(
            "Y plane size mismatch: expected {expected_y}, got {}",
            y.len()
        )));
    }
    if u.len() != expected_uv {
        return Err(Error::invalid_argument(format!(
            "U plane size mismatch: expected {expected_uv}, got {}",
            u.len()
        )));
    }
    if v.len() != expected_uv {
        return Err(Error::invalid_argument(format!(
            "V plane size mismatch: expected {expected_uv}, got {}",
            v.len()
        )));
    }
    Ok(())
}

/// 入力検証: stride 付き NV12 フレームデータのサイズを検証する。
pub fn validate_nv12_strided(
    y: &[u8],
    uv: &[u8],
    width: i32,
    height: i32,
    y_pitch: i32,
    uv_pitch: i32,
) -> Result<()> {
    if width <= 0 || height <= 0 {
        return Err(Error::invalid_argument("width and height must be positive"));
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(Error::invalid_argument(format!(
            "dimensions too large: {width}x{height} (max {MAX_DIMENSION})"
        )));
    }
    if width % 2 != 0 || height % 2 != 0 {
        return Err(Error::invalid_argument(
            "NV12 requires even width and height",
        ));
    }
    if y_pitch < width {
        return Err(Error::invalid_argument(format!(
            "y_pitch ({y_pitch}) must be >= width ({width})"
        )));
    }
    if uv_pitch < width {
        return Err(Error::invalid_argument(format!(
            "uv_pitch ({uv_pitch}) must be >= width ({width})"
        )));
    }
    let h = height as usize;
    let expected_y = y_pitch as usize * h;
    let expected_uv = uv_pitch as usize * (h / 2);
    if y.len() != expected_y {
        return Err(Error::invalid_argument(format!(
            "Y plane size mismatch: expected {expected_y}, got {}",
            y.len()
        )));
    }
    if uv.len() != expected_uv {
        return Err(Error::invalid_argument(format!(
            "UV plane size mismatch: expected {expected_uv}, got {}",
            uv.len()
        )));
    }
    Ok(())
}

/// 入力検証: stride 付き YUY2 フレームデータのサイズを検証する。
pub fn validate_yuy2_strided(data: &[u8], width: i32, height: i32, pitch: i32) -> Result<()> {
    if width <= 0 || height <= 0 {
        return Err(Error::invalid_argument("width and height must be positive"));
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(Error::invalid_argument(format!(
            "dimensions too large: {width}x{height} (max {MAX_DIMENSION})"
        )));
    }
    if width % 2 != 0 {
        return Err(Error::invalid_argument("YUY2 requires even width"));
    }
    if pitch < width * 2 {
        return Err(Error::invalid_argument(format!(
            "pitch ({pitch}) must be >= width*2 ({})",
            width * 2
        )));
    }
    let expected = pitch as usize * height as usize;
    if data.len() != expected {
        return Err(Error::invalid_argument(format!(
            "YUY2 data size mismatch: expected {expected}, got {}",
            data.len()
        )));
    }
    Ok(())
}

/// 入力検証: YUY2 フレームデータのサイズを検証する。
pub fn validate_yuy2(data: &[u8], width: i32, height: i32) -> Result<()> {
    if width <= 0 || height <= 0 {
        return Err(Error::invalid_argument("width and height must be positive"));
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(Error::invalid_argument(format!(
            "dimensions too large: {width}x{height} (max {MAX_DIMENSION})"
        )));
    }
    if width % 2 != 0 {
        return Err(Error::invalid_argument("YUY2 requires even width"));
    }
    let expected = width as usize * height as usize * 2;
    if data.len() != expected {
        return Err(Error::invalid_argument(format!(
            "YUY2 data size mismatch: expected {expected}, got {}",
            data.len()
        )));
    }
    Ok(())
}

/// 入力検証: 4bpp パックドフォーマット (RGBA/BGRA) のフレームデータサイズを検証する。
fn validate_packed_4bpp(data: &[u8], width: i32, height: i32, format_name: &str) -> Result<()> {
    if width <= 0 || height <= 0 {
        return Err(Error::invalid_argument("width and height must be positive"));
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(Error::invalid_argument(format!(
            "dimensions too large: {width}x{height} (max {MAX_DIMENSION})"
        )));
    }
    let expected = width as usize * height as usize * 4;
    if data.len() != expected {
        return Err(Error::invalid_argument(format!(
            "{format_name} data size mismatch: expected {expected}, got {}",
            data.len()
        )));
    }
    Ok(())
}

/// 入力検証: RGBA フレームデータのサイズを検証する。
pub fn validate_rgba(data: &[u8], width: i32, height: i32) -> Result<()> {
    validate_packed_4bpp(data, width, height, "RGBA")
}

/// 入力検証: BGRA フレームデータのサイズを検証する。
pub fn validate_bgra(data: &[u8], width: i32, height: i32) -> Result<()> {
    validate_packed_4bpp(data, width, height, "BGRA")
}

#[derive(Debug, Clone)]
pub struct VideoPlayerStats {
    pub video_queue_size: usize,
    pub audio_queue_ms: f64,
    pub dropped_frames: i64,
    pub repeated_frames: i64,
    pub video_pts_us: i64,
    pub audio_pts_us: i64,
    pub sync_diff_us: i64,
    pub current_video_width: i32,
    pub current_video_height: i32,
    pub current_fps: f32,
    pub total_frames_enqueued: i64,
    pub total_frames_rendered: i64,
    pub video_buffer_ms: f64,
    pub elapsed_time_ms: f64,
    pub video_bitrate_kbps: f64,
    pub avg_texture_update_us: u64,
    pub max_texture_update_us: u64,
    pub avg_clear_copy_us: u64,
    pub avg_present_us: u64,
    pub avg_vsync_interval_us: u64,
}

struct VideoPlayerInner {
    // ウィンドウ/レンダリング (drop 順序: texture → renderer → window)
    texture: Option<Texture>,
    renderer: Renderer,
    window: Window,
    window_width: i32,
    window_height: i32,
    texture_width: i32,
    texture_height: i32,
    texture_format: VideoFormat,
    title: String,

    // 映像キュー
    video_queue: VecDeque<VideoFrame>,

    // 同期状態
    playing: bool,
    has_played: bool,
    last_video_pts_us: i64,

    // 映像のみタイミング
    video_start_time_ns: u64,
    first_video_pts_us: i64,
    video_only_started: bool,

    // 同期設定
    sync_threshold_us: i64,
    max_video_queue_size: usize,
    vsync_interval: i32,

    // 統計
    dropped_frames: i64,
    repeated_frames: i64,
    total_frames_enqueued: i64,
    total_frames_rendered: i64,
    play_start_time_ns: u64,
    last_frame_size_bytes: i64,

    // FPS 計算
    fps_calc_start_ns: u64,
    fps_frame_count: i32,
    current_fps: f32,

    // render 計測 (累積)
    render_texture_update_us: u64,
    render_clear_copy_us: u64,
    render_present_us: u64,
    render_count: u64,
    // VSync 間隔計測
    last_present_end_ns: u64,
    render_vsync_interval_us: u64,
    render_vsync_count: u64,
    // tex upload の最大値
    render_tex_max_us: u64,

    // UI
    show_stats_overlay: bool,
    key_callback: Option<Arc<dyn Fn(u32) -> bool + Send + Sync + 'static>>,

    // 状態
    open: bool,
}

pub struct VideoPlayer {
    audio: AudioPlayer,
    inner: Mutex<VideoPlayerInner>,
}

impl VideoPlayer {
    pub fn new(width: i32, height: i32, title: &str) -> Result<Self> {
        crate::init()?;

        let window = Window::new(title, width, height)?;
        let mut renderer = Renderer::new_gpu(&window)?;
        renderer.set_vsync(1)?;

        Ok(Self {
            audio: AudioPlayer::new(),
            inner: Mutex::new(VideoPlayerInner {
                texture: None,
                renderer,
                window,
                window_width: width,
                window_height: height,
                texture_width: 0,
                texture_height: 0,
                texture_format: VideoFormat::I420,
                title: title.to_string(),

                video_queue: VecDeque::new(),

                playing: false,
                has_played: false,
                last_video_pts_us: 0,

                video_start_time_ns: 0,
                first_video_pts_us: 0,
                video_only_started: false,

                sync_threshold_us: 40_000,
                max_video_queue_size: 5,
                vsync_interval: 1,

                dropped_frames: 0,
                repeated_frames: 0,
                total_frames_enqueued: 0,
                total_frames_rendered: 0,
                play_start_time_ns: 0,
                last_frame_size_bytes: 0,

                fps_calc_start_ns: 0,
                fps_frame_count: 0,
                current_fps: 0.0,

                render_texture_update_us: 0,
                render_clear_copy_us: 0,
                render_present_us: 0,
                render_count: 0,
                last_present_end_ns: 0,
                render_vsync_interval_us: 0,
                render_vsync_count: 0,
                render_tex_max_us: 0,

                show_stats_overlay: false,
                key_callback: None,

                open: true,
            }),
        })
    }

    /// I420 フレームをキューに追加する。
    pub fn enqueue_video_i420(
        &self,
        y: &[u8],
        u: &[u8],
        v: &[u8],
        width: i32,
        height: i32,
        pts_us: i64,
    ) -> Result<()> {
        validate_i420(y, u, v, width, height)?;
        self.enqueue_frame(VideoFrame {
            pts_us,
            width,
            height,
            y_pitch: width,
            uv_pitch: width / 2,
            format: VideoFormat::I420,
            data: FrameData::Planar {
                y: y.to_vec(),
                u: u.to_vec(),
                v: v.to_vec(),
            },
        })
    }

    /// stride 付き I420 フレームをキューに追加する。
    #[allow(clippy::too_many_arguments)]
    pub fn enqueue_video_i420_strided(
        &self,
        y: &[u8],
        u: &[u8],
        v: &[u8],
        width: i32,
        height: i32,
        y_pitch: i32,
        uv_pitch: i32,
        pts_us: i64,
    ) -> Result<()> {
        validate_i420_strided(y, u, v, width, height, y_pitch, uv_pitch)?;
        self.enqueue_frame(VideoFrame {
            pts_us,
            width,
            height,
            y_pitch,
            uv_pitch,
            format: VideoFormat::I420,
            data: FrameData::Planar {
                y: y.to_vec(),
                u: u.to_vec(),
                v: v.to_vec(),
            },
        })
    }

    /// NV12 フレームをキューに追加する。
    pub fn enqueue_video_nv12(
        &self,
        y: &[u8],
        uv: &[u8],
        width: i32,
        height: i32,
        pts_us: i64,
    ) -> Result<()> {
        validate_nv12(y, uv, width, height)?;
        self.enqueue_frame(VideoFrame {
            pts_us,
            width,
            height,
            y_pitch: width,
            uv_pitch: width,
            format: VideoFormat::NV12,
            data: FrameData::SemiPlanar {
                y: y.to_vec(),
                uv: uv.to_vec(),
            },
        })
    }

    /// stride 付き NV12 フレームをキューに追加する。
    #[allow(clippy::too_many_arguments)]
    pub fn enqueue_video_nv12_strided(
        &self,
        y: &[u8],
        uv: &[u8],
        width: i32,
        height: i32,
        y_pitch: i32,
        uv_pitch: i32,
        pts_us: i64,
    ) -> Result<()> {
        validate_nv12_strided(y, uv, width, height, y_pitch, uv_pitch)?;
        self.enqueue_frame(VideoFrame {
            pts_us,
            width,
            height,
            y_pitch,
            uv_pitch,
            format: VideoFormat::NV12,
            data: FrameData::SemiPlanar {
                y: y.to_vec(),
                uv: uv.to_vec(),
            },
        })
    }

    /// YUY2 フレームをキューに追加する。
    pub fn enqueue_video_yuy2(
        &self,
        data: &[u8],
        width: i32,
        height: i32,
        pts_us: i64,
    ) -> Result<()> {
        validate_yuy2(data, width, height)?;
        self.enqueue_frame(VideoFrame {
            pts_us,
            width,
            height,
            y_pitch: width * 2,
            uv_pitch: 0,
            format: VideoFormat::YUY2,
            data: FrameData::Packed(data.to_vec()),
        })
    }

    /// stride 付き YUY2 フレームをキューに追加する。
    pub fn enqueue_video_yuy2_strided(
        &self,
        data: &[u8],
        width: i32,
        height: i32,
        pitch: i32,
        pts_us: i64,
    ) -> Result<()> {
        validate_yuy2_strided(data, width, height, pitch)?;
        self.enqueue_frame(VideoFrame {
            pts_us,
            width,
            height,
            y_pitch: pitch,
            uv_pitch: 0,
            format: VideoFormat::YUY2,
            data: FrameData::Packed(data.to_vec()),
        })
    }

    /// RGBA フレームをキューに追加する。
    pub fn enqueue_video_rgba(
        &self,
        data: &[u8],
        width: i32,
        height: i32,
        pts_us: i64,
    ) -> Result<()> {
        validate_rgba(data, width, height)?;
        self.enqueue_frame(VideoFrame {
            pts_us,
            width,
            height,
            y_pitch: width * 4,
            uv_pitch: 0,
            format: VideoFormat::Rgba,
            data: FrameData::Packed(data.to_vec()),
        })
    }

    /// BGRA フレームをキューに追加する。
    pub fn enqueue_video_bgra(
        &self,
        data: &[u8],
        width: i32,
        height: i32,
        pts_us: i64,
    ) -> Result<()> {
        validate_bgra(data, width, height)?;
        self.enqueue_frame(VideoFrame {
            pts_us,
            width,
            height,
            y_pitch: width * 4,
            uv_pitch: 0,
            format: VideoFormat::Bgra,
            data: FrameData::Packed(data.to_vec()),
        })
    }

    /// BGRA フレームをキューに追加する (所有権を移動、コピーなし)。
    pub fn enqueue_video_bgra_owned(
        &self,
        data: Vec<u8>,
        width: i32,
        height: i32,
        pts_us: i64,
    ) -> Result<()> {
        validate_bgra(&data, width, height)?;
        self.enqueue_frame(VideoFrame {
            pts_us,
            width,
            height,
            y_pitch: width * 4,
            uv_pitch: 0,
            format: VideoFormat::Bgra,
            data: FrameData::Packed(data),
        })
    }

    /// CVPixelBuffer を直接キューに追加する (macOS ゼロコピー)。
    ///
    /// `pixel_buffer_ptr` は `PixelBuffer::as_ptr()` から取得した CVPixelBuffer ポインタ。
    /// 内部で CFRetain するため、呼び出し元はこの関数の後にポインタ元を drop してよい。
    ///
    /// # Safety
    ///
    /// `pixel_buffer_ptr` は有効な CVPixelBuffer へのポインタでなければならない。
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn enqueue_video_pixel_buffer(
        &self,
        pixel_buffer_ptr: *mut std::ffi::c_void,
        format: VideoFormat,
        width: i32,
        height: i32,
        y_pitch: i32,
        uv_pitch: i32,
        pts_us: i64,
    ) -> Result<()> {
        if width <= 0 || height <= 0 {
            return Err(Error::invalid_argument("width and height must be positive"));
        }
        if width > MAX_DIMENSION || height > MAX_DIMENSION {
            return Err(Error::invalid_argument(format!(
                "dimensions too large: {width}x{height} (max {MAX_DIMENSION})"
            )));
        }
        let pixel_buffer_ref = unsafe { PixelBufferRef::from_ptr(pixel_buffer_ptr)? };
        self.enqueue_frame(VideoFrame {
            pts_us,
            width,
            height,
            y_pitch,
            uv_pitch,
            format,
            data: FrameData::PixelBuffer(pixel_buffer_ref),
        })
    }

    /// 音声データをキューに追加する。
    pub fn enqueue_audio(
        &self,
        data: &[u8],
        pts_us: i64,
        sample_rate: i32,
        channels: i32,
        format: AudioFormat,
    ) -> Result<()> {
        self.audio
            .enqueue_audio(data, pts_us, sample_rate, channels, format)
    }

    /// 再生を開始する。
    pub fn play(&self) -> Result<()> {
        self.audio.play()?;
        let mut inner = self.inner.lock().unwrap();
        if !inner.playing {
            inner.playing = true;
            inner.has_played = true;
            if inner.play_start_time_ns == 0 {
                inner.play_start_time_ns = unsafe { ffi::SDL_GetTicksNS() };
                inner.fps_calc_start_ns = 0;
            }
        }
        Ok(())
    }

    /// 再生を一時停止する。
    pub fn pause(&self) -> Result<()> {
        self.audio.pause()?;
        let mut inner = self.inner.lock().unwrap();
        inner.playing = false;
        Ok(())
    }

    /// 再生を停止してキューをクリアする。
    pub fn stop(&self) -> Result<()> {
        self.audio.stop()?;
        let mut inner = self.inner.lock().unwrap();
        inner.playing = false;
        inner.has_played = false;
        inner.video_queue.clear();
        inner.last_video_pts_us = 0;
        inner.video_only_started = false;
        inner.video_start_time_ns = 0;
        inner.first_video_pts_us = 0;
        inner.dropped_frames = 0;
        inner.repeated_frames = 0;
        inner.total_frames_enqueued = 0;
        inner.total_frames_rendered = 0;
        inner.play_start_time_ns = 0;
        inner.last_frame_size_bytes = 0;
        inner.fps_calc_start_ns = 0;
        inner.fps_frame_count = 0;
        inner.current_fps = 0.0;
        Ok(())
    }

    /// イベント処理とフレームレンダリングを行う。ウィンドウが閉じられた場合 false を返す。
    pub fn poll_events(&self) -> Result<bool> {
        // inner ロック前に音声処理
        self.audio.process()?;
        let audio_clock_us = self.audio.audio_clock_us();
        let audio_started = self.audio.is_started();

        let mut inner = self.inner.lock().unwrap();

        if !inner.open {
            return Ok(false);
        }

        // イベント処理 (Mutex を一時解放)
        drop(inner);
        let mut should_close = false;
        let mut key_events = Vec::new();
        let mut resize_event = None;
        while let Some(event) = poll_event() {
            match event {
                Event::Quit | Event::WindowClose => {
                    should_close = true;
                }
                Event::KeyDown { keycode } => {
                    key_events.push(keycode);
                }
                Event::WindowResized { width, height } => {
                    resize_event = Some((width, height));
                }
                _ => {}
            }
        }
        inner = self.inner.lock().unwrap();

        if let Some((w, h)) = resize_event {
            inner.window_width = w;
            inner.window_height = h;
        }

        if should_close {
            inner.open = false;
            return Ok(false);
        }

        // キーイベント処理 (S キーのトグルはロック内で処理)
        for &keycode in &key_events {
            if keycode == KEYCODE_S {
                inner.show_stats_overlay = !inner.show_stats_overlay;
            }
        }

        // コールバックはロック外で呼ぶ (デッドロック防止・poison 連鎖防止)
        let callback = inner.key_callback.clone();
        drop(inner);
        if let Some(ref callback) = callback {
            for keycode in key_events {
                if !callback(keycode) {
                    let mut inner = self.inner.lock().unwrap();
                    inner.open = false;
                    return Ok(false);
                }
            }
        }
        let mut inner = self.inner.lock().unwrap();

        // フレームレンダリング
        if inner.playing {
            Self::render_next_frame(&mut inner, audio_clock_us, audio_started)?;
        }

        Ok(true)
    }

    /// ウィンドウを閉じる。
    pub fn close(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.inner.lock().unwrap().open
    }

    pub fn is_playing(&self) -> bool {
        self.inner.lock().unwrap().playing
    }

    pub fn width(&self) -> i32 {
        self.inner.lock().unwrap().window_width
    }

    pub fn height(&self) -> i32 {
        self.inner.lock().unwrap().window_height
    }

    pub fn title(&self) -> String {
        self.inner.lock().unwrap().title.clone()
    }

    pub fn set_title(&self, title: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.window.set_title(title)?;
        inner.title = title.to_string();
        Ok(())
    }

    pub fn renderer_name(&self) -> String {
        self.inner.lock().unwrap().renderer.name().to_string()
    }

    pub fn volume(&self) -> f32 {
        self.audio.volume()
    }

    pub fn set_volume(&self, volume: f32) -> Result<()> {
        self.audio.set_volume(volume)
    }

    pub fn set_key_callback<F>(&self, callback: Option<F>)
    where
        F: Fn(u32) -> bool + Send + Sync + 'static,
    {
        let mut inner = self.inner.lock().unwrap();
        inner.key_callback =
            callback.map(|f| Arc::new(f) as Arc<dyn Fn(u32) -> bool + Send + Sync>);
    }

    pub fn stats(&self) -> VideoPlayerStats {
        // audio → inner の順でロックする（play/pause/stop と同じ順序）
        let audio_clock = self.audio.audio_clock_us();
        let audio_queue_ms = self.audio.audio_queue_ms();

        let inner = self.inner.lock().unwrap();

        let video_buffer_ms = if inner.video_queue.len() >= 2 {
            let first_pts = inner.video_queue.front().unwrap().pts_us;
            let last_pts = inner.video_queue.back().unwrap().pts_us;
            (last_pts - first_pts) as f64 / 1000.0
        } else {
            0.0
        };

        let elapsed_ms = if inner.has_played {
            let now = unsafe { ffi::SDL_GetTicksNS() };
            (now - inner.play_start_time_ns) as f64 / 1_000_000.0
        } else {
            0.0
        };

        let video_bitrate_kbps = if inner.current_fps > 0.0 {
            inner.last_frame_size_bytes as f64 * inner.current_fps as f64 * 8.0 / 1000.0
        } else {
            0.0
        };

        VideoPlayerStats {
            video_queue_size: inner.video_queue.len(),
            audio_queue_ms,
            dropped_frames: inner.dropped_frames,
            repeated_frames: inner.repeated_frames,
            video_pts_us: inner.last_video_pts_us,
            audio_pts_us: audio_clock,
            sync_diff_us: audio_clock - inner.last_video_pts_us,
            current_video_width: inner.texture_width,
            current_video_height: inner.texture_height,
            current_fps: inner.current_fps,
            total_frames_enqueued: inner.total_frames_enqueued,
            total_frames_rendered: inner.total_frames_rendered,
            video_buffer_ms,
            elapsed_time_ms: elapsed_ms,
            video_bitrate_kbps,
            avg_texture_update_us: inner
                .render_texture_update_us
                .checked_div(inner.render_count)
                .unwrap_or(0),
            max_texture_update_us: inner.render_tex_max_us,
            avg_clear_copy_us: inner
                .render_clear_copy_us
                .checked_div(inner.render_count)
                .unwrap_or(0),
            avg_present_us: inner
                .render_present_us
                .checked_div(inner.render_count)
                .unwrap_or(0),
            avg_vsync_interval_us: inner
                .render_vsync_interval_us
                .checked_div(inner.render_vsync_count)
                .unwrap_or(0),
        }
    }

    pub fn set_max_video_queue_size(&self, size: usize) {
        self.inner.lock().unwrap().max_video_queue_size = size;
    }

    pub fn max_video_queue_size(&self) -> usize {
        self.inner.lock().unwrap().max_video_queue_size
    }

    /// 映像キューをクリアし、タイミング状態をリセットする。
    pub fn drain_video(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.video_queue.clear();
        inner.video_only_started = false;
        inner.video_start_time_ns = 0;
        inner.first_video_pts_us = 0;
    }

    pub fn set_stats_overlay(&self, enabled: bool) {
        self.inner.lock().unwrap().show_stats_overlay = enabled;
    }

    pub fn stats_overlay(&self) -> bool {
        self.inner.lock().unwrap().show_stats_overlay
    }

    /// VSync を設定する (0: 無効, 1: 有効)。
    pub fn set_vsync(&self, interval: i32) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.vsync_interval = interval;
        inner.renderer.set_vsync(interval)
    }

    // --- 内部メソッド ---

    fn enqueue_frame(&self, frame: VideoFrame) -> Result<()> {
        let frame_size_bytes = frame.data.size_bytes() as i64;

        let mut inner = self.inner.lock().unwrap();

        if inner.has_played && !inner.playing {
            return Err(Error::NotPlaying);
        }

        inner.last_frame_size_bytes = frame_size_bytes;
        inner.total_frames_enqueued += 1;

        if inner.max_video_queue_size > 0 {
            while inner.video_queue.len() >= inner.max_video_queue_size {
                inner.video_queue.pop_front();
                inner.dropped_frames += 1;
            }
        }

        inner.video_queue.push_back(frame);
        Ok(())
    }

    fn render_next_frame(
        inner: &mut VideoPlayerInner,
        audio_clock_us: i64,
        audio_started: bool,
    ) -> Result<()> {
        if !inner.playing {
            return Ok(());
        }

        if inner.video_queue.is_empty() {
            return Ok(());
        }

        // クロック取得
        let clock_us = if audio_started {
            audio_clock_us
        } else {
            if !inner.video_only_started {
                inner.video_start_time_ns = unsafe { ffi::SDL_GetTicksNS() };
                inner.first_video_pts_us = inner.video_queue.front().unwrap().pts_us;
                inner.video_only_started = true;
            }
            let elapsed_ns = unsafe { ffi::SDL_GetTicksNS() } - inner.video_start_time_ns;
            let elapsed_us = elapsed_ns as i64 / 1000;
            inner.first_video_pts_us + elapsed_us
        };

        // AV 同期
        loop {
            if inner.video_queue.is_empty() {
                break;
            }

            let diff = inner.video_queue.front().unwrap().pts_us - clock_us;

            if diff < -inner.sync_threshold_us {
                // フレームが遅すぎる → ドロップ
                inner.video_queue.pop_front();
                inner.dropped_frames += 1;
                continue;
            }

            if diff > inner.sync_threshold_us {
                // フレームが早すぎる → リピート
                inner.repeated_frames += 1;
                break;
            }

            // 範囲内 → レンダリング
            let frame = inner.video_queue.pop_front().unwrap();
            Self::render_frame_internal(inner, &frame, audio_clock_us)?;
            break;
        }

        Ok(())
    }

    fn render_frame_internal(
        inner: &mut VideoPlayerInner,
        frame: &VideoFrame,
        audio_clock_us: i64,
    ) -> Result<()> {
        // テクスチャの再作成が必要か
        if inner.texture.is_none()
            || frame.width != inner.texture_width
            || frame.height != inner.texture_height
            || frame.format != inner.texture_format
        {
            Self::create_texture(inner, frame.width, frame.height, frame.format)?;
        }

        // テクスチャ更新
        let tex_start = std::time::Instant::now();
        if let Some(ref mut texture) = inner.texture {
            match &frame.data {
                FrameData::Planar { y, u, v } => {
                    texture.update_yuv(y, frame.y_pitch, u, frame.uv_pitch, v, frame.uv_pitch)?;
                }
                FrameData::SemiPlanar { y, uv } => {
                    texture.update_nv12(y, frame.y_pitch, uv, frame.uv_pitch)?;
                }
                FrameData::Packed(data) => {
                    texture.update_packed(data, frame.y_pitch)?;
                }
                FrameData::PixelBuffer(pb) => {
                    let lock = pb.lock()?;
                    let h = frame.height as usize;
                    match frame.format {
                        VideoFormat::NV12 => {
                            let y = lock.plane(0)?;
                            let uv = lock.plane(1)?;
                            let y_pitch = lock.stride(0)?;
                            let uv_pitch = lock.stride(1)?;
                            // SDL はテクスチャの高さ分読み取るため、
                            // 実プレーンがそれ以上の行数を持つことを検証する
                            let chroma_h = h.div_ceil(2);
                            if lock.plane_height(0) < h || lock.plane_height(1) < chroma_h {
                                return Err(Error::invalid_argument(format!(
                                    "NV12 PixelBuffer plane height insufficient: Y={}, UV={}, required Y>={h}, UV>={chroma_h}",
                                    lock.plane_height(0),
                                    lock.plane_height(1),
                                )));
                            }
                            if (y_pitch as usize) < frame.width as usize
                                || (uv_pitch as usize) < frame.width as usize
                            {
                                return Err(Error::invalid_argument(format!(
                                    "NV12 PixelBuffer stride insufficient: Y={y_pitch}, UV={uv_pitch}, required >= {}",
                                    frame.width,
                                )));
                            }
                            texture.update_nv12(y, y_pitch, uv, uv_pitch)?;
                        }
                        VideoFormat::I420 => {
                            let y = lock.plane(0)?;
                            let u = lock.plane(1)?;
                            let v = lock.plane(2)?;
                            let y_pitch = lock.stride(0)?;
                            let uv_pitch = lock.stride(1)?;
                            let chroma_h = h.div_ceil(2);
                            let half_w = (frame.width as usize).div_ceil(2);
                            if lock.plane_height(0) < h
                                || lock.plane_height(1) < chroma_h
                                || lock.plane_height(2) < chroma_h
                            {
                                return Err(Error::invalid_argument(format!(
                                    "I420 PixelBuffer plane height insufficient: Y={}, U={}, V={}, required Y>={h}, UV>={chroma_h}",
                                    lock.plane_height(0),
                                    lock.plane_height(1),
                                    lock.plane_height(2),
                                )));
                            }
                            if (y_pitch as usize) < frame.width as usize
                                || (uv_pitch as usize) < half_w
                            {
                                return Err(Error::invalid_argument(format!(
                                    "I420 PixelBuffer stride insufficient: Y={y_pitch}, UV={uv_pitch}, required Y>={}, UV>={half_w}",
                                    frame.width,
                                )));
                            }
                            texture.update_yuv(y, y_pitch, u, uv_pitch, v, uv_pitch)?;
                        }
                        _ => {
                            return Err(Error::invalid_argument(format!(
                                "PixelBuffer does not support format: {}",
                                frame.format.name()
                            )));
                        }
                    }
                }
            }
        }
        let tex_us = tex_start.elapsed().as_micros() as u64;
        inner.render_texture_update_us += tex_us;
        if tex_us > inner.render_tex_max_us {
            inner.render_tex_max_us = tex_us;
        }

        // レンダリング
        let cc_start = std::time::Instant::now();
        inner.renderer.set_draw_color(0, 0, 0, 255)?;
        inner.renderer.clear()?;
        if let Some(ref texture) = inner.texture {
            inner.renderer.copy(texture)?;
        }

        // 統計オーバーレイ
        if inner.show_stats_overlay {
            Self::render_stats_overlay(inner, audio_clock_us)?;
        }
        inner.render_clear_copy_us += cc_start.elapsed().as_micros() as u64;

        let present_start = std::time::Instant::now();
        inner.renderer.present()?;
        let present_end_ns = unsafe { ffi::SDL_GetTicksNS() };
        inner.render_present_us += present_start.elapsed().as_micros() as u64;
        inner.render_count += 1;

        // VSync 間隔計測
        if inner.last_present_end_ns > 0 {
            let interval_us = (present_end_ns - inner.last_present_end_ns) / 1000;
            inner.render_vsync_interval_us += interval_us;
            inner.render_vsync_count += 1;
        }
        inner.last_present_end_ns = present_end_ns;

        // 追跡更新
        inner.last_video_pts_us = frame.pts_us;
        inner.total_frames_rendered += 1;

        // FPS 計算
        inner.fps_frame_count += 1;
        let now_ns = unsafe { ffi::SDL_GetTicksNS() };
        if inner.fps_calc_start_ns == 0 {
            inner.fps_calc_start_ns = now_ns;
        }
        let elapsed_ns = now_ns - inner.fps_calc_start_ns;
        if elapsed_ns >= 1_000_000_000 {
            inner.current_fps = inner.fps_frame_count as f32 * 1_000_000_000.0 / elapsed_ns as f32;
            inner.fps_calc_start_ns = now_ns;
            inner.fps_frame_count = 0;
        }

        Ok(())
    }

    fn create_texture(
        inner: &mut VideoPlayerInner,
        width: i32,
        height: i32,
        format: VideoFormat,
    ) -> Result<()> {
        inner.texture = None;

        let texture = Texture::new(&inner.renderer, format, width, height)?;
        inner.texture = Some(texture);
        inner.texture_width = width;
        inner.texture_height = height;
        inner.texture_format = format;

        inner.renderer.set_logical_presentation(width, height)?;
        inner.renderer.set_vsync(inner.vsync_interval)?;

        Ok(())
    }

    fn render_stats_overlay(inner: &mut VideoPlayerInner, audio_clock_us: i64) -> Result<()> {
        // スケール保存・設定
        let (orig_sx, orig_sy) = inner.renderer.scale()?;

        // 半透明背景
        inner.renderer.set_draw_blend_mode(crate::BLENDMODE_BLEND)?;
        inner.renderer.set_draw_color(0, 0, 0, 128)?;
        inner.renderer.set_scale(1.0, 1.0)?;
        let (output_w, output_h) = inner.renderer.output_size()?;
        let overlay_h = 200.0_f32.min(output_h as f32);
        inner
            .renderer
            .fill_rect(0.0, 0.0, output_w as f32, overlay_h)?;

        // テキスト描画 (2x スケール)
        inner.renderer.set_scale(2.0, 2.0)?;
        inner.renderer.set_draw_color(0, 255, 0, 255)?;

        let char_h = crate::DEBUG_TEXT_FONT_CHARACTER_SIZE as f32;
        let mut y = 4.0;
        let x = 4.0;

        let renderer_name = inner.renderer.name();
        let lines = [
            format!("Renderer: {renderer_name}"),
            format!(
                "Format: {} {}x{}",
                inner.texture_format.name(),
                inner.texture_width,
                inner.texture_height
            ),
            format!(
                "FPS: {:.1}  Bitrate: {:.0} kbps",
                inner.current_fps,
                if inner.current_fps > 0.0 {
                    inner.last_frame_size_bytes as f64 * inner.current_fps as f64 * 8.0 / 1000.0
                } else {
                    0.0
                }
            ),
            format!(
                "Elapsed: {:.1}s",
                if inner.has_played {
                    let now = unsafe { ffi::SDL_GetTicksNS() };
                    (now - inner.play_start_time_ns) as f64 / 1_000_000_000.0
                } else {
                    0.0
                }
            ),
            format!(
                "Frames: enq={} rend={} drop={} rep={} queue={}",
                inner.total_frames_enqueued,
                inner.total_frames_rendered,
                inner.dropped_frames,
                inner.repeated_frames,
                inner.video_queue.len()
            ),
            format!(
                "Video buffer: {:.1}ms",
                if inner.video_queue.len() >= 2 {
                    let first = inner.video_queue.front().unwrap().pts_us;
                    let last = inner.video_queue.back().unwrap().pts_us;
                    (last - first) as f64 / 1000.0
                } else {
                    0.0
                }
            ),
            format!(
                "Video PTS: {}us  Audio PTS: {}us",
                inner.last_video_pts_us, audio_clock_us
            ),
            format!(
                "AV sync: {:.1}ms",
                (audio_clock_us - inner.last_video_pts_us) as f64 / 1000.0
            ),
        ];

        for line in &lines {
            inner.renderer.debug_text(x, y, line)?;
            y += char_h + 2.0;
        }

        // スケール復元
        inner.renderer.set_scale(orig_sx, orig_sy)?;

        Ok(())
    }
}
