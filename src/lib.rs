//! raw_player - PTS ベースの raw 映像/音声プレイヤー

mod audio_format;
mod audio_player;
mod audio_stream;
mod error;
mod event;
mod ffi;
mod pixel_buffer;
mod renderer;
mod texture;
mod video_format;
mod video_frame;
mod video_player;
mod window;

pub use audio_format::AudioFormat;
pub use audio_player::{AudioPlayer, AudioPlayerStats};
pub use error::{Error, Result};
pub use event::{Event, KEYCODE_ESCAPE, KEYCODE_S, poll_event};
pub use pixel_buffer::PixelBufferRef;
pub use renderer::Renderer;
pub use texture::Texture;
pub use video_format::VideoFormat;
pub use video_player::{
    VideoPlayer, VideoPlayerStats, validate_bgra, validate_i420, validate_i420_strided,
    validate_nv12, validate_nv12_strided, validate_pixel_buffer_enqueue, validate_rgba,
    validate_yuy2, validate_yuy2_strided,
};
pub use window::Window;

use std::sync::Mutex;

/// ビルド時に参照したリポジトリ URL
pub const BUILD_REPOSITORY: &str = ffi::BUILD_METADATA_REPOSITORY;

/// ビルド時に参照したリポジトリのバージョン（タグ）
pub const BUILD_VERSION: &str = ffi::BUILD_METADATA_VERSION;

/// ブレンドモード: アルファブレンド。
#[allow(clippy::unnecessary_cast)]
pub const BLENDMODE_BLEND: u32 = ffi::SDL_BLENDMODE_BLEND as u32;

/// デバッグテキストのフォント文字サイズ (ピクセル)。
pub const DEBUG_TEXT_FONT_CHARACTER_SIZE: i32 = ffi::SDL_DEBUG_TEXT_FONT_CHARACTER_SIZE as i32;

/// `SDL_Init` / `SDL_Quit` と整合する初期化フラグ。並行する `init` / `quit` を直列化する。
static SDL_INITIALIZED: Mutex<bool> = Mutex::new(false);

/// SDL3 を初期化する。
///
/// SDL3 は初期化時に、SIGINT / SIGTERM のハンドラが既定 (`SIG_DFL`) のままなら、
/// それらを SDL の終了イベント (`SDL_EVENT_QUIT`) に変換するシグナルハンドラを登録する。
/// SDL のイベントを処理しないアプリではこの終了イベントが処理されず、
/// SIGTERM などで終了できなくなる。そのようなアプリは [`init_without_signal_handlers`] を使うこと。
///
/// 複数回呼び出しても安全。別スレッドから同時に初回のみ呼び出した場合も、`SDL_Init` が完了するまでブロックされる。
pub fn init() -> Result<()> {
    init_impl(false)
}

/// SDL3 のシグナルハンドラを登録せずに初期化する。
///
/// SDL3 は既定で、SIGINT / SIGTERM のハンドラが既定 (`SIG_DFL`) のままなら、
/// それらを SDL の終了イベント (`SDL_EVENT_QUIT`) に変換するシグナルハンドラを登録する。
/// SDL のイベントを処理しないアプリではこの終了イベントが処理されず、
/// `timeout` コマンド、systemd、コンテナの停止が送る SIGTERM で終了できなくなる。
/// この関数はそのハンドラを登録せずに SDL3 を初期化する。
///
/// SDL の初期化より前に環境変数 `SDL_NO_SIGNAL_HANDLERS=1` を設定するのと同等。
///
/// 既に [`init`] などで初期化済みの場合は何もせず `Ok(())` を返す。
pub fn init_without_signal_handlers() -> Result<()> {
    init_impl(true)
}

/// SDL3 を初期化する。`without_signal_handlers` が true のときは SDL のシグナルハンドラを無効化する。
fn init_impl(without_signal_handlers: bool) -> Result<()> {
    // SDL_Init はプロセス全体の状態を扱うため、初回呼び出しを Mutex で直列化する。
    let mut initialized = SDL_INITIALIZED
        .lock()
        .expect("SDL init state mutex poisoned");
    if *initialized {
        return Ok(());
    }
    if without_signal_handlers {
        // SDL は SDL_Init の中でヒントを参照してシグナルハンドラの登録可否を決めるため、
        // SDL_Init より前にヒントを設定する必要がある。
        let name = ffi::SDL_HINT_NO_SIGNAL_HANDLERS.as_ptr().cast();
        if !unsafe { ffi::SDL_SetHint(name, c"1".as_ptr()) } {
            return Err(Error::from_sdl());
        }
    }
    #[allow(clippy::unnecessary_cast)]
    let flags = ffi::SDL_INIT_VIDEO as u32 | ffi::SDL_INIT_AUDIO as u32;
    if unsafe { ffi::SDL_Init(flags) } {
        *initialized = true;
        Ok(())
    } else {
        Err(Error::from_sdl())
    }
}

/// SDL3 を終了する。
///
/// # Safety
///
/// すべての SDL リソース (VideoPlayer, Window, Renderer 等) を drop した後に呼ぶこと。
/// リソースが残った状態で呼ぶと、drop 時に解放済みリソースへアクセスしクラッシュする。
pub unsafe fn quit() {
    let mut initialized = SDL_INITIALIZED
        .lock()
        .expect("SDL init state mutex poisoned");
    if *initialized {
        unsafe { ffi::SDL_Quit() };
        *initialized = false;
    }
}

/// Raw FFI バインディングへのアクセス。
pub mod sys {
    pub use crate::ffi::*;
}

/// Linux のみ: 並行初回 `init()` が `Mutex` で直列化されることの退行防止。
#[cfg(all(test, target_os = "linux"))]
mod init_concurrency_tests {
    use std::sync::{Arc, Barrier};
    use std::thread;

    use crate::{init, quit};

    #[test]
    fn init_serializes_concurrent_first_calls() {
        // Safety: テスト開始時点で SDL リソースは存在しない
        unsafe { quit() };

        unsafe {
            // テストプロセス内・スレッド生成前のみ。他テストとのデータ競合を避けるため unsafe。
            std::env::set_var("SDL_VIDEODRIVER", "dummy");
        }

        let barrier = Arc::new(Barrier::new(2));
        let b1 = Arc::clone(&barrier);
        let b2 = Arc::clone(&barrier);
        let h1 = thread::spawn(move || {
            b1.wait();
            init().is_ok()
        });
        let h2 = thread::spawn(move || {
            b2.wait();
            init().is_ok()
        });
        assert!(
            h1.join().unwrap(),
            "first thread init failed (SDL unavailable?)"
        );
        assert!(h2.join().unwrap(), "second thread init failed");
        // Safety: テスト完了後のクリーンアップ、SDL リソースは存在しない
        unsafe { quit() };
    }
}
