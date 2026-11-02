//! 映像ピクセルフォーマットの定義。

use crate::sys;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoFormat {
    I420,
    NV12,
    YUY2,
    Rgba,
    Bgra,
}

impl VideoFormat {
    // bindgen は環境によって enum 定数の基底型を i32 / u32 に分けるため、
    // as u32 ではなくビット列を経由して u32 に揃える。
    pub(crate) fn to_sdl_pixel_format(self) -> u32 {
        let format = match self {
            Self::I420 => sys::SDL_PixelFormat_SDL_PIXELFORMAT_IYUV,
            Self::NV12 => sys::SDL_PixelFormat_SDL_PIXELFORMAT_NV12,
            Self::YUY2 => sys::SDL_PixelFormat_SDL_PIXELFORMAT_YUY2,
            Self::Rgba => sys::SDL_PixelFormat_SDL_PIXELFORMAT_RGBA8888,
            // BGRA はメモリ上のバイト順 (B,G,R,A)。リトルエンディアンでは
            // 32bit 値として読むと 0xAARRGGBB = ARGB となるため ARGB8888 を使う。
            Self::Bgra => sys::SDL_PixelFormat_SDL_PIXELFORMAT_ARGB8888,
        };
        u32::from_ne_bytes(format.to_ne_bytes())
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::I420 => "I420",
            Self::NV12 => "NV12",
            Self::YUY2 => "YUY2",
            Self::Rgba => "RGBA",
            Self::Bgra => "BGRA",
        }
    }
}
