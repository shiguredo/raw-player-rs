use crate::ffi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoFormat {
    I420,
    NV12,
    YUY2,
    Rgba,
    Bgra,
}

impl VideoFormat {
    // bindgen は Windows (MSVC) で enum 定数を i32、Linux (GCC) で u32 として
    // 生成するため、as u32 で統一する
    #[allow(clippy::unnecessary_cast)]
    pub(crate) fn to_sdl_pixel_format(self) -> u32 {
        match self {
            Self::I420 => ffi::SDL_PixelFormat_SDL_PIXELFORMAT_IYUV as u32,
            Self::NV12 => ffi::SDL_PixelFormat_SDL_PIXELFORMAT_NV12 as u32,
            Self::YUY2 => ffi::SDL_PixelFormat_SDL_PIXELFORMAT_YUY2 as u32,
            Self::Rgba => ffi::SDL_PixelFormat_SDL_PIXELFORMAT_RGBA8888 as u32,
            // BGRA はメモリ上のバイト順 (B,G,R,A)。リトルエンディアンでは
            // 32bit 値として読むと 0xAARRGGBB = ARGB となるため ARGB8888 を使う。
            Self::Bgra => ffi::SDL_PixelFormat_SDL_PIXELFORMAT_ARGB8888 as u32,
        }
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
