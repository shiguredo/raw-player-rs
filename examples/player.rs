//! カメラ映像とマイク音声をキャプチャして再生するサンプル
//!
//! shiguredo_video_device でカメラ映像を取得し、
//! shiguredo_audio_device でマイク音声を取得し、
//! raw_player::VideoPlayer で AV 同期再生する。
//!
//! ```
//! cargo run --example player -- --list-devices
//! cargo run --example player
//! cargo run --example player -- --resolution 1080p --fps 60
//! ```

use std::process;
use std::sync::mpsc;
use std::time::Instant;

use raw_player::{AudioFormat, KEYCODE_ESCAPE, VideoPlayer};
use shiguredo_audio_device::{
    AudioCapture, AudioCaptureConfig, AudioDeviceList, AudioFormat as DeviceAudioFormat,
    AudioFrameOwned,
};
use shiguredo_video_device::{
    PixelBuffer, PixelFormat, VideoCapture, VideoCaptureConfig, VideoDeviceList, VideoFrame,
    VideoFrameOwned,
};

/// pixel_buffer のみを保持する軽量なフレーム情報 (data コピー不要)。
struct PixelBufferFrame {
    pixel_buffer: PixelBuffer,
    pixel_format: PixelFormat,
    width: i32,
    height: i32,
    stride: i32,
    stride_uv: i32,
    timestamp_us: i64,
}

enum CaptureMessage {
    /// stride 付きデータのコピー (pixel_buffer がない場合)
    Video(VideoFrameOwned),
    /// CVPixelBuffer を直接保持 (macOS ゼロコピー)
    VideoPixelBuffer(PixelBufferFrame),
    Audio(AudioFrameOwned),
}

struct Args {
    list_devices: bool,
    video_input_device: Option<String>,
    audio_input_device: Option<String>,
    width: i32,
    height: i32,
    fps: i32,
    sample_rate: i32,
    channels: i32,
    duration: Option<f64>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            list_devices: false,
            video_input_device: None,
            audio_input_device: None,
            width: 1280,
            height: 720,
            fps: 30,
            sample_rate: 48000,
            channels: 1,
            duration: None,
        }
    }
}

fn print_usage() {
    eprintln!(
        "Usage: player [OPTIONS]

Options:
  --list-devices              デバイス一覧を表示して終了
  --video-input-device <id>   映像入力デバイス ID
  --audio-input-device <id>   音声入力デバイス ID
  --resolution <value>        解像度 (720p, 1080p, 4k, WxH) [default: 720p]
  --fps <n>                   フレームレート [default: 30]
  --sample-rate <n>           音声サンプルレート [default: 48000]
  --channels <n>              音声チャンネル数 [default: 1]
  --duration <sec>            再生時間 (秒)
  -h, --help                  ヘルプを表示"
    );
}

fn parse_resolution(s: &str) -> Option<(i32, i32)> {
    match s.to_lowercase().as_str() {
        "4k" | "2160p" => Some((3840, 2160)),
        "1080p" => Some((1920, 1080)),
        "720p" => Some((1280, 720)),
        "540p" => Some((960, 540)),
        _ => {
            let parts: Vec<&str> = s.split('x').collect();
            if parts.len() == 2 {
                let w = parts[0].parse().ok()?;
                let h = parts[1].parse().ok()?;
                Some((w, h))
            } else {
                None
            }
        }
    }
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    let mut result = Args::default();

    while i < args.len() {
        match args[i].as_str() {
            "--list-devices" => result.list_devices = true,
            "--video-input-device" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("エラー: --video-input-device に値が必要です");
                    std::process::exit(1);
                }
                result.video_input_device = Some(args[i].clone());
            }
            "--audio-input-device" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("エラー: --audio-input-device に値が必要です");
                    std::process::exit(1);
                }
                result.audio_input_device = Some(args[i].clone());
            }
            "--resolution" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("エラー: --resolution に値が必要です");
                    std::process::exit(1);
                }
                if let Some((w, h)) = parse_resolution(&args[i]) {
                    result.width = w;
                    result.height = h;
                } else {
                    eprintln!("エラー: 不正な解像度: {}", args[i]);
                    std::process::exit(1);
                }
            }
            "--fps" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("エラー: --fps に値が必要です");
                    std::process::exit(1);
                }
                result.fps = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("エラー: 不正な fps: {}", args[i]);
                    std::process::exit(1);
                });
            }
            "--sample-rate" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("エラー: --sample-rate に値が必要です");
                    std::process::exit(1);
                }
                result.sample_rate = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("エラー: 不正な sample-rate: {}", args[i]);
                    std::process::exit(1);
                });
            }
            "--channels" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("エラー: --channels に値が必要です");
                    std::process::exit(1);
                }
                result.channels = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("エラー: 不正な channels: {}", args[i]);
                    std::process::exit(1);
                });
            }
            "--duration" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("エラー: --duration に値が必要です");
                    std::process::exit(1);
                }
                result.duration = Some(args[i].parse().unwrap_or_else(|_| {
                    eprintln!("エラー: 不正な duration: {}", args[i]);
                    std::process::exit(1);
                }));
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                eprintln!("エラー: 不明な引数: {other}");
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }
    result
}

fn map_audio_format(format: DeviceAudioFormat) -> AudioFormat {
    match format {
        DeviceAudioFormat::S16 => AudioFormat::S16,
        DeviceAudioFormat::F32 => AudioFormat::F32,
    }
}

fn list_devices() {
    println!("=== 映像デバイス一覧 ===");
    match VideoDeviceList::enumerate() {
        Ok(devices) => {
            if devices.is_empty() {
                println!("  映像デバイスが見つかりません");
            } else {
                for device in devices.devices() {
                    let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
                    let id = device.unique_id().unwrap_or_else(|_| "Unknown".to_string());
                    println!("  {name}");
                    println!("    ID: {id}");
                    for fmt in device.formats() {
                        println!(
                            "    {}x{} @ {:.0}-{:.0} fps ({})",
                            fmt.width,
                            fmt.height,
                            fmt.min_fps,
                            fmt.max_fps,
                            fmt.pixel_format.name()
                        );
                    }
                }
            }
        }
        Err(e) => eprintln!("  映像デバイスの列挙に失敗: {e:?}"),
    }

    println!();

    println!("=== 音声入力デバイス一覧 ===");
    match AudioDeviceList::enumerate_input() {
        Ok(devices) => {
            if devices.is_empty() {
                println!("  音声入力デバイスが見つかりません");
            } else {
                for device in devices.devices() {
                    let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
                    let id = device.unique_id().unwrap_or_else(|_| "Unknown".to_string());
                    println!(
                        "  {name} ({} ch, {} Hz)",
                        device.channels(),
                        device.sample_rate()
                    );
                    println!("    ID: {id}");
                }
            }
        }
        Err(e) => eprintln!("  音声入力デバイスの列挙に失敗: {e:?}"),
    }
}

/// PixelFormat を raw_player::VideoFormat に変換する。
fn map_video_format(pf: PixelFormat) -> Option<raw_player::VideoFormat> {
    match pf {
        PixelFormat::Nv12 => Some(raw_player::VideoFormat::NV12),
        PixelFormat::I420 => Some(raw_player::VideoFormat::I420),
        PixelFormat::Yuy2 => Some(raw_player::VideoFormat::YUY2),
        PixelFormat::Unknown(_) => None,
    }
}

/// VideoFrame の PixelFormat に応じて適切な enqueue メソッドを呼び出す。
/// pixel_buffer がある場合は CVPixelBuffer をそのまま渡してゼロコピーする。
/// ない場合は stride 付きデータをコピーして渡す。
fn enqueue_video_frame(player: &VideoPlayer, frame: &VideoFrame<'_>) -> raw_player::Result<()> {
    let pts_us = frame.timestamp_us;

    // macOS: pixel_buffer がある場合はゼロコピーパス
    if let Some(ref pb) = frame.pixel_buffer {
        let Some(format) = map_video_format(frame.pixel_format) else {
            return Ok(());
        };
        return unsafe {
            player.enqueue_video_pixel_buffer(
                pb.as_ptr(),
                format,
                frame.width,
                frame.height,
                frame.stride,
                frame.stride_uv,
                pts_us,
            )
        };
    }

    // フォールバック: stride 付きデータをコピーして渡す
    match frame.pixel_format {
        PixelFormat::Nv12 => {
            let Some(uv_data) = frame.uv_data else {
                return Ok(());
            };
            player.enqueue_video_nv12_strided(
                frame.data,
                uv_data,
                frame.width,
                frame.height,
                frame.stride,
                frame.stride_uv,
                pts_us,
            )?;
        }
        PixelFormat::I420 => {
            let Some(uv_data) = frame.uv_data else {
                return Ok(());
            };
            let uv_h = frame.height as usize / 2;
            let u_plane_size = frame.stride_uv as usize * uv_h;
            player.enqueue_video_i420_strided(
                frame.data,
                &uv_data[..u_plane_size],
                &uv_data[u_plane_size..],
                frame.width,
                frame.height,
                frame.stride,
                frame.stride_uv,
                pts_us,
            )?;
        }
        PixelFormat::Yuy2 => {
            player.enqueue_video_yuy2_strided(
                frame.data,
                frame.width,
                frame.height,
                frame.stride,
                pts_us,
            )?;
        }
        PixelFormat::Unknown(_) => {
            use std::sync::Once;
            static WARN: Once = Once::new();
            WARN.call_once(|| {
                eprintln!(
                    "警告: 未対応のピクセルフォーマット: {}",
                    frame.pixel_format.name()
                );
            });
        }
    }
    Ok(())
}

fn main() {
    let args = parse_args();

    if args.list_devices {
        list_devices();
        return;
    }

    // キャプチャスレッドからメインスレッドへフレームを送るチャネル
    let (tx, rx) = mpsc::channel::<CaptureMessage>();

    // VideoCapture を作成
    let video_config = VideoCaptureConfig {
        device_id: args.video_input_device,
        width: args.width,
        height: args.height,
        fps: args.fps,
        pixel_format: None,
    };
    let video_tx = tx.clone();
    let mut video_capture = match VideoCapture::new(video_config, move |frame: VideoFrame<'_>| {
        if let Some(pb) = frame.pixel_buffer.clone() {
            // macOS: pixel_buffer があればデータコピーなしで送信
            let _ = video_tx.send(CaptureMessage::VideoPixelBuffer(PixelBufferFrame {
                pixel_buffer: pb,
                pixel_format: frame.pixel_format,
                width: frame.width,
                height: frame.height,
                stride: frame.stride,
                stride_uv: frame.stride_uv,
                timestamp_us: frame.timestamp_us,
            }));
        } else {
            // pixel_buffer がない場合はデータをコピーして送信
            let _ = video_tx.send(CaptureMessage::Video(frame.to_owned()));
        }
    }) {
        Ok(capture) => capture,
        Err(e) => {
            eprintln!("VideoCapture の作成に失敗しました: {e:?}");
            process::exit(1);
        }
    };

    // AudioCapture を作成 (失敗した場合は映像のみで続行)
    let audio_config = AudioCaptureConfig {
        device_id: args.audio_input_device,
        sample_rate: args.sample_rate,
        channels: args.channels,
    };
    let audio_tx = tx;
    let mut audio_capture = match AudioCapture::new(audio_config, move |frame| {
        let _ = audio_tx.send(CaptureMessage::Audio(frame.to_owned()));
    }) {
        Ok(capture) => Some(capture),
        Err(e) => {
            eprintln!("音声デバイスの初期化に失敗 ({e:?}), 映像のみで動作します");
            None
        }
    };

    // VideoPlayer を作成
    let title = format!("Player ({}x{} @ {} fps)", args.width, args.height, args.fps);
    let player = match VideoPlayer::new(args.width, args.height, &title) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("VideoPlayer の作成に失敗しました: {e:?}");
            process::exit(1);
        }
    };

    println!("=== Player ===");
    println!("解像度: {}x{}", args.width, args.height);
    println!("FPS: {}", args.fps);
    println!("GPU Renderer: {}", player.renderer_name());
    if let Some(ref ac) = audio_capture {
        println!("音声: {} Hz, {} ch", ac.sample_rate(), ac.channels());
    }
    if let Some(duration) = args.duration {
        println!("再生時間: {duration} 秒");
    }

    // ESC キーで終了
    player.set_key_callback(Some(|keycode: u32| -> bool { keycode != KEYCODE_ESCAPE }));

    // キャプチャ開始
    if let Err(e) = video_capture.start() {
        eprintln!("映像キャプチャの開始に失敗しました: {e:?}");
        process::exit(1);
    }
    if let Some(ref mut ac) = audio_capture
        && let Err(e) = ac.start()
    {
        eprintln!("音声キャプチャの開始に失敗 ({e:?}), 映像のみで動作します");
        audio_capture = None;
    }

    // 再生開始
    if let Err(e) = player.play() {
        eprintln!("再生の開始に失敗しました: {e:?}");
        process::exit(1);
    }

    println!();
    println!("ESC キーで終了, S キーで統計オーバーレイ切替");
    println!();

    let start = Instant::now();

    // メインループ
    loop {
        // キャプチャスレッドからのフレームを受信して enqueue
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CaptureMessage::Video(owned) => {
                    let frame = owned.as_frame();
                    if let Err(e) = enqueue_video_frame(&player, &frame) {
                        eprintln!("映像フレームの enqueue に失敗: {e}");
                    }
                }
                CaptureMessage::VideoPixelBuffer(pbf) => {
                    let Some(format) = map_video_format(pbf.pixel_format) else {
                        continue;
                    };
                    if let Err(e) = unsafe {
                        player.enqueue_video_pixel_buffer(
                            pbf.pixel_buffer.as_ptr(),
                            format,
                            pbf.width,
                            pbf.height,
                            pbf.stride,
                            pbf.stride_uv,
                            pbf.timestamp_us,
                        )
                    } {
                        eprintln!("映像フレームの enqueue に失敗: {e}");
                    }
                }
                CaptureMessage::Audio(owned) => {
                    let format = map_audio_format(owned.format);
                    if let Err(e) = player.enqueue_audio(
                        &owned.data,
                        owned.timestamp_us,
                        owned.sample_rate,
                        owned.channels,
                        format,
                    ) {
                        eprintln!("音声データの enqueue に失敗: {e}");
                    }
                }
            }
        }

        // SDL イベント処理とフレームレンダリング
        match player.poll_events() {
            Ok(true) => {}
            _ => break,
        }

        if let Some(duration) = args.duration
            && start.elapsed().as_secs_f64() >= duration
        {
            break;
        }
    }

    // 停止
    video_capture.stop();
    if let Some(ref mut ac) = audio_capture {
        ac.stop();
    }

    // 統計表示
    let stats = player.stats();
    let elapsed = start.elapsed().as_secs_f64();

    println!();
    println!("=== 統計 ===");
    println!("時間: {elapsed:.2} 秒");
    println!(
        "映像フレーム: enqueue={}, render={}, drop={}, repeat={}",
        stats.total_frames_enqueued,
        stats.total_frames_rendered,
        stats.dropped_frames,
        stats.repeated_frames,
    );
    println!("FPS: {:.1}", stats.current_fps);
    println!("AV 同期差: {:.1} ms", stats.sync_diff_us as f64 / 1000.0);

    // SDL リソースをすべて解放してから SDL を終了する
    drop(player);
    // Safety: player は直前の drop(player) で解放済み
    unsafe { raw_player::quit() };
    println!("完了");
}
