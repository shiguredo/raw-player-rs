use crate::error::{Error, Result};
use crate::ffi;
use crate::renderer::Renderer;
use crate::video_format::VideoFormat;
use std::ptr::NonNull;

/// pitch 下限の `width * 4` 相当が i32 で溢れないようにする上限。
const MAX_DIMENSION: i32 = i32::MAX / 4;

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
        self.validate_dimensions_for_update()?;
        if self.format != VideoFormat::I420 {
            return Err(Error::invalid_argument(
                "update_yuv requires I420 texture format",
            ));
        }
        if self.width % 2 != 0 || self.height % 2 != 0 {
            return Err(Error::invalid_argument(
                "I420 requires even width and height",
            ));
        }

        let chroma_w = self.width / 2;
        let chroma_h = self.height / 2;
        Self::ensure_pitch_at_least(y_pitch, self.width, "y_pitch")?;
        Self::ensure_pitch_at_least(u_pitch, chroma_w, "u_pitch")?;
        Self::ensure_pitch_at_least(v_pitch, chroma_w, "v_pitch")?;
        Self::ensure_plane_len(y_plane, y_pitch, self.height, "Y")?;
        Self::ensure_plane_len(u_plane, u_pitch, chroma_h, "U")?;
        Self::ensure_plane_len(v_plane, v_pitch, chroma_h, "V")?;

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
        self.validate_dimensions_for_update()?;
        if self.format != VideoFormat::NV12 {
            return Err(Error::invalid_argument(
                "update_nv12 requires NV12 texture format",
            ));
        }
        if self.width % 2 != 0 || self.height % 2 != 0 {
            return Err(Error::invalid_argument(
                "NV12 requires even width and height",
            ));
        }

        let chroma_h = self.height / 2;
        Self::ensure_pitch_at_least(y_pitch, self.width, "y_pitch")?;
        Self::ensure_pitch_at_least(uv_pitch, self.width, "uv_pitch")?;
        Self::ensure_plane_len(y_plane, y_pitch, self.height, "Y")?;
        Self::ensure_plane_len(uv_plane, uv_pitch, chroma_h, "UV")?;

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
        self.validate_dimensions_for_update()?;
        let bytes_per_pixel = match self.format {
            VideoFormat::YUY2 => {
                if self.width % 2 != 0 {
                    return Err(Error::invalid_argument("YUY2 requires even width"));
                }
                2i32
            }
            VideoFormat::Rgba | VideoFormat::Bgra => 4i32,
            VideoFormat::I420 | VideoFormat::NV12 => {
                return Err(Error::invalid_argument(
                    "update_packed requires YUY2, RGBA, or BGRA texture format",
                ));
            }
        };

        let min_pitch = self
            .width
            .checked_mul(bytes_per_pixel)
            .ok_or_else(|| Error::invalid_argument("pitch lower bound overflow"))?;
        Self::ensure_pitch_at_least(pitch, min_pitch, "pitch")?;
        Self::ensure_plane_len(data, pitch, self.height, "packed")?;

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

    /// update 入口で非正・過大寸法を拒否する（長さ計算前ガード）。
    fn validate_dimensions_for_update(&self) -> Result<()> {
        if self.width <= 0 || self.height <= 0 {
            return Err(Error::invalid_argument(format!(
                "texture dimensions must be positive: {}x{}",
                self.width, self.height
            )));
        }
        if self.width > MAX_DIMENSION || self.height > MAX_DIMENSION {
            return Err(Error::invalid_argument(format!(
                "dimensions too large: {}x{} (max {MAX_DIMENSION})",
                self.width, self.height
            )));
        }
        Ok(())
    }

    /// pitch が正かつ行バイト下限以上であることを保証する。
    fn ensure_pitch_at_least(pitch: i32, min_pitch: i32, name: &str) -> Result<()> {
        if pitch <= 0 {
            return Err(Error::invalid_argument(format!(
                "{name} must be positive: {pitch}"
            )));
        }
        if pitch < min_pitch {
            return Err(Error::invalid_argument(format!(
                "{name} too small: {pitch} < {min_pitch}"
            )));
        }
        Ok(())
    }

    /// プレーン長が `pitch * rows` 以上であることを保証する。
    fn ensure_plane_len(plane: &[u8], pitch: i32, rows: i32, name: &str) -> Result<()> {
        let pitch_usize = usize::try_from(pitch).map_err(|_| {
            Error::invalid_argument(format!(
                "{name} pitch is not representable as usize: {pitch}"
            ))
        })?;
        let rows_usize = usize::try_from(rows).map_err(|_| {
            Error::invalid_argument(format!("{name} rows is not representable as usize: {rows}"))
        })?;
        let min_len = pitch_usize.checked_mul(rows_usize).ok_or_else(|| {
            Error::invalid_argument(format!("{name} plane minimum length overflow"))
        })?;
        if plane.len() < min_len {
            return Err(Error::invalid_argument(format!(
                "{name} plane too short: {} < {min_len}",
                plane.len()
            )));
        }
        Ok(())
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { ffi::SDL_DestroyTexture(self.raw.as_ptr()) };
    }
}
