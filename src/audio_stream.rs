use crate::audio_format::AudioFormat;
use crate::error::{Error, Result};
use crate::ffi;
use std::ptr::NonNull;

/// SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK (0xFFFFFFFF)
/// bindgen は SDL3 の `#define` マクロ定数を生成しないため手動定義。
const SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK: ffi::SDL_AudioDeviceID = 0xFFFF_FFFF;

pub(crate) struct AudioStream {
    raw: NonNull<ffi::SDL_AudioStream>,
}

impl AudioStream {
    /// デフォルト再生デバイスの音声ストリームを開く。
    pub(crate) fn open(sample_rate: i32, channels: i32, format: AudioFormat) -> Result<Self> {
        let spec = ffi::SDL_AudioSpec {
            format: format.to_sdl(),
            channels,
            freq: sample_rate,
        };
        let raw = unsafe {
            ffi::SDL_OpenAudioDeviceStream(
                SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK,
                &spec,
                None,
                std::ptr::null_mut(),
            )
        };
        NonNull::new(raw)
            .map(|raw| Self { raw })
            .ok_or_else(Error::from_sdl)
    }

    /// PCM データをストリームに書き込む。
    pub(crate) fn put_data(&mut self, data: &[u8]) -> Result<()> {
        let len = i32::try_from(data.len())
            .map_err(|_| Error::invalid_argument("data length exceeds i32::MAX"))?;
        if unsafe { ffi::SDL_PutAudioStreamData(self.raw.as_ptr(), data.as_ptr().cast(), len) } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    /// キューに残っているバイト数を返す。
    pub(crate) fn queued_bytes(&self) -> i32 {
        let bytes = unsafe { ffi::SDL_GetAudioStreamQueued(self.raw.as_ptr()) };
        if bytes < 0 { 0 } else { bytes }
    }

    /// ゲイン (音量) を設定する。
    pub(crate) fn set_gain(&mut self, gain: f32) -> Result<()> {
        if unsafe { ffi::SDL_SetAudioStreamGain(self.raw.as_ptr(), gain) } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    /// 再生を再開する。
    pub(crate) fn resume(&mut self) -> Result<()> {
        if unsafe { ffi::SDL_ResumeAudioStreamDevice(self.raw.as_ptr()) } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    /// 再生を一時停止する。
    pub(crate) fn pause(&mut self) -> Result<()> {
        if unsafe { ffi::SDL_PauseAudioStreamDevice(self.raw.as_ptr()) } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    /// キューをクリアする。
    pub(crate) fn clear(&mut self) -> Result<()> {
        if unsafe { ffi::SDL_ClearAudioStream(self.raw.as_ptr()) } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }
}

impl Drop for AudioStream {
    fn drop(&mut self) {
        unsafe { ffi::SDL_DestroyAudioStream(self.raw.as_ptr()) };
    }
}

// SAFETY: SDL_AudioStream は内部でスレッドセーフに管理されており、
// SDL API 経由でのみアクセスするため、別スレッドへの移動は安全。
unsafe impl Send for AudioStream {}
