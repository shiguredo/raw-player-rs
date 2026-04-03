use crate::error::{Error, Result};
use crate::ffi;
use crate::renderer::Renderer;
use crate::video_format::VideoFormat;
use std::ptr::NonNull;

pub struct Texture {
    raw: NonNull<ffi::SDL_Texture>,
    width: i32,
    height: i32,
    format: VideoFormat,
}

impl Texture {
    /// 指定フォーマットのストリーミングテクスチャを作成する。
    pub fn new(renderer: &Renderer, format: VideoFormat, width: i32, height: i32) -> Result<Self> {
        let raw = unsafe {
            ffi::SDL_CreateTexture(
                renderer.as_ptr(),
                format.to_sdl_pixel_format() as _,
                ffi::SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING as _,
                width,
                height,
            )
        };
        NonNull::new(raw)
            .map(|raw| Self {
                raw,
                width,
                height,
                format,
            })
            .ok_or_else(Error::from_sdl)
    }

    /// I420 (YUV420) 形式のテクスチャを作成する。
    pub fn new_yuv(renderer: &Renderer, width: i32, height: i32) -> Result<Self> {
        Self::new(renderer, VideoFormat::I420, width, height)
    }

    pub fn as_ptr(&self) -> *mut ffi::SDL_Texture {
        self.raw.as_ptr()
    }

    /// I420 データでテクスチャを更新する。
    pub fn update_yuv(
        &mut self,
        y_plane: &[u8],
        y_pitch: i32,
        u_plane: &[u8],
        u_pitch: i32,
        v_plane: &[u8],
        v_pitch: i32,
    ) -> Result<()> {
        if unsafe {
            ffi::SDL_UpdateYUVTexture(
                self.raw.as_ptr(),
                std::ptr::null(),
                y_plane.as_ptr(),
                y_pitch,
                u_plane.as_ptr(),
                u_pitch,
                v_plane.as_ptr(),
                v_pitch,
            )
        } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    /// NV12 データでテクスチャを更新する。
    pub fn update_nv12(
        &mut self,
        y_plane: &[u8],
        y_pitch: i32,
        uv_plane: &[u8],
        uv_pitch: i32,
    ) -> Result<()> {
        if unsafe {
            ffi::SDL_UpdateNVTexture(
                self.raw.as_ptr(),
                std::ptr::null(),
                y_plane.as_ptr(),
                y_pitch,
                uv_plane.as_ptr(),
                uv_pitch,
            )
        } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    /// パックドフォーマット (YUY2/RGBA/BGRA) のデータでテクスチャを更新する。
    pub fn update_packed(&mut self, data: &[u8], pitch: i32) -> Result<()> {
        if unsafe {
            ffi::SDL_UpdateTexture(
                self.raw.as_ptr(),
                std::ptr::null(),
                data.as_ptr().cast(),
                pitch,
            )
        } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn format(&self) -> VideoFormat {
        self.format
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { ffi::SDL_DestroyTexture(self.raw.as_ptr()) };
    }
}
