use std::ffi::{CStr, CString};
use std::ptr::NonNull;

use crate::error::{Error, Result};
use crate::ffi;
use crate::texture::Texture;
use crate::window::Window;

pub struct Renderer {
    raw: NonNull<ffi::SDL_Renderer>,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        let raw = unsafe { ffi::SDL_CreateRenderer(window.as_ptr(), std::ptr::null()) };
        NonNull::new(raw)
            .map(|raw| Self { raw })
            .ok_or_else(Error::from_sdl)
    }

    /// 指定名のレンダラーを作成する。
    pub fn new_with_name(window: &Window, name: &str) -> Result<Self> {
        let c_name =
            CString::new(name).map_err(|_| Error::invalid_argument("name contains null byte"))?;
        let raw = unsafe { ffi::SDL_CreateRenderer(window.as_ptr(), c_name.as_ptr()) };
        NonNull::new(raw)
            .map(|raw| Self { raw })
            .ok_or_else(Error::from_sdl)
    }

    /// GPU レンダラーを作成する。失敗時はプラットフォーム固有バックエンドにフォールバック。
    pub fn new_gpu(window: &Window) -> Result<Self> {
        // まず "gpu" を試す
        if let Ok(r) = Self::new_with_name(window, "gpu") {
            return Ok(r);
        }

        // プラットフォーム固有フォールバック
        #[cfg(target_os = "macos")]
        {
            if let Ok(r) = Self::new_with_name(window, "metal") {
                return Ok(r);
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(r) = Self::new_with_name(window, "vulkan") {
                return Ok(r);
            }
        }

        // 最終フォールバック: デフォルト
        Self::new(window)
    }

    pub fn as_ptr(&self) -> *mut ffi::SDL_Renderer {
        self.raw.as_ptr()
    }

    pub fn clear(&mut self) -> Result<()> {
        if unsafe { ffi::SDL_RenderClear(self.raw.as_ptr()) } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    pub fn copy(&mut self, texture: &Texture) -> Result<()> {
        if unsafe {
            ffi::SDL_RenderTexture(
                self.raw.as_ptr(),
                texture.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
            )
        } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    pub fn present(&mut self) -> Result<()> {
        if unsafe { ffi::SDL_RenderPresent(self.raw.as_ptr()) } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    pub fn set_draw_color(&mut self, r: u8, g: u8, b: u8, a: u8) -> Result<()> {
        if unsafe { ffi::SDL_SetRenderDrawColor(self.raw.as_ptr(), r, g, b, a) } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    pub fn set_draw_blend_mode(&mut self, mode: u32) -> Result<()> {
        if unsafe { ffi::SDL_SetRenderDrawBlendMode(self.raw.as_ptr(), mode as _) } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32) -> Result<()> {
        let rect = ffi::SDL_FRect { x, y, w, h };
        if unsafe { ffi::SDL_RenderFillRect(self.raw.as_ptr(), &rect) } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    pub fn debug_text(&mut self, x: f32, y: f32, text: &str) -> Result<()> {
        let c_text =
            CString::new(text).map_err(|_| Error::invalid_argument("text contains null byte"))?;
        if unsafe { ffi::SDL_RenderDebugText(self.raw.as_ptr(), x, y, c_text.as_ptr()) } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    pub fn output_size(&self) -> Result<(i32, i32)> {
        let mut w = 0;
        let mut h = 0;
        if unsafe { ffi::SDL_GetRenderOutputSize(self.raw.as_ptr(), &mut w, &mut h) } {
            Ok((w, h))
        } else {
            Err(Error::from_sdl())
        }
    }

    pub fn scale(&self) -> Result<(f32, f32)> {
        let mut sx = 0.0f32;
        let mut sy = 0.0f32;
        if unsafe { ffi::SDL_GetRenderScale(self.raw.as_ptr(), &mut sx, &mut sy) } {
            Ok((sx, sy))
        } else {
            Err(Error::from_sdl())
        }
    }

    pub fn set_scale(&mut self, scale_x: f32, scale_y: f32) -> Result<()> {
        if unsafe { ffi::SDL_SetRenderScale(self.raw.as_ptr(), scale_x, scale_y) } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    /// レターボックス付き論理プレゼンテーションを設定する。
    pub fn set_logical_presentation(&mut self, width: i32, height: i32) -> Result<()> {
        if unsafe {
            ffi::SDL_SetRenderLogicalPresentation(
                self.raw.as_ptr(),
                width,
                height,
                ffi::SDL_RendererLogicalPresentation_SDL_LOGICAL_PRESENTATION_LETTERBOX as _,
            )
        } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    /// VSync を設定する。
    pub fn set_vsync(&mut self, interval: i32) -> Result<()> {
        if unsafe { ffi::SDL_SetRenderVSync(self.raw.as_ptr(), interval) } {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    pub fn name(&self) -> &str {
        let ptr = unsafe { ffi::SDL_GetRendererName(self.raw.as_ptr()) };
        if ptr.is_null() {
            return "unknown";
        }
        unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or("unknown")
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe { ffi::SDL_DestroyRenderer(self.raw.as_ptr()) };
    }
}

// SAFETY: SDL_Renderer は作成元スレッド以外からの操作を想定していないが、
// Mutex<VideoPlayerInner> 内に保持し排他アクセスを保証しているため Send は安全。
unsafe impl Send for Renderer {}
