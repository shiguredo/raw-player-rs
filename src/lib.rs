//! raw_player - PTS ベースの raw 映像/音声プレイヤー

mod audio_format;
mod audio_player;
mod audio_stream;
mod error;
mod event;
mod ffi;
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
pub use renderer::Renderer;
pub use texture::Texture;
pub use video_format::VideoFormat;
pub use video_player::{
    VideoPlayer, VideoPlayerStats, validate_bgra, validate_i420, validate_nv12, validate_rgba,
    validate_yuy2,
};
pub use window::Window;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{AcqRel, Release};

/// ブレンドモード: アルファブレンド。
#[allow(clippy::unnecessary_cast)]
pub const BLENDMODE_BLEND: u32 = ffi::SDL_BLENDMODE_BLEND as u32;

/// デバッグテキストのフォント文字サイズ (ピクセル)。
pub const DEBUG_TEXT_FONT_CHARACTER_SIZE: i32 = ffi::SDL_DEBUG_TEXT_FONT_CHARACTER_SIZE as i32;

static SDL_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// SDL3 を初期化する。
/// 複数回呼び出しても安全。
pub fn init() -> Result<()> {
    if SDL_INITIALIZED.swap(true, AcqRel) {
        return Ok(());
    }
    #[allow(clippy::unnecessary_cast)]
    let flags = ffi::SDL_INIT_VIDEO as u32 | ffi::SDL_INIT_AUDIO as u32;
    if unsafe { ffi::SDL_Init(flags) } {
        Ok(())
    } else {
        SDL_INITIALIZED.store(false, Release);
        Err(Error::from_sdl())
    }
}

/// SDL3 を終了する。
///
/// すべての SDL リソース (VideoPlayer, Window, Renderer 等) を drop した後に呼ぶこと。
/// リソースが残った状態で呼ぶと、drop 時に解放済みリソースへアクセスしクラッシュする。
pub fn quit() {
    if SDL_INITIALIZED.swap(false, AcqRel) {
        unsafe { ffi::SDL_Quit() };
    }
}

/// Raw FFI バインディングへのアクセス。
pub mod sys {
    pub use crate::ffi::*;
}
