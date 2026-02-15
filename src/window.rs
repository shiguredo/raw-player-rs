use crate::error::{Error, Result};
use crate::ffi;
use std::ffi::CString;
use std::ptr::NonNull;

// bindgen は SDL3 の `#define` マクロ定数を生成しないため手動定義。
const SDL_WINDOW_RESIZABLE: u64 = 0x0000_0000_0000_0020;
const SDL_WINDOW_HIGH_PIXEL_DENSITY: u64 = 0x0000_0000_0000_2000;

pub struct Window {
    raw: NonNull<ffi::SDL_Window>,
}

impl Window {
    pub fn new(title: &str, width: i32, height: i32) -> Result<Self> {
        let c_title =
            CString::new(title).map_err(|_| Error::invalid_argument("title contains null byte"))?;
        let flags = SDL_WINDOW_RESIZABLE | SDL_WINDOW_HIGH_PIXEL_DENSITY;
        let raw = unsafe { ffi::SDL_CreateWindow(c_title.as_ptr(), width, height, flags) };
        NonNull::new(raw)
            .map(|raw| Self { raw })
            .ok_or_else(Error::from_sdl)
    }

    pub fn as_ptr(&self) -> *mut ffi::SDL_Window {
        self.raw.as_ptr()
    }

    pub fn size(&self) -> (i32, i32) {
        let mut w = 0;
        let mut h = 0;
        unsafe { ffi::SDL_GetWindowSize(self.raw.as_ptr(), &mut w, &mut h) };
        (w, h)
    }

    pub fn set_size(&mut self, width: i32, height: i32) -> Result<()> {
        let result = unsafe { ffi::SDL_SetWindowSize(self.raw.as_ptr(), width, height) };
        if result {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    pub fn set_title(&mut self, title: &str) -> Result<()> {
        let c_title =
            CString::new(title).map_err(|_| Error::invalid_argument("title contains null byte"))?;
        let result = unsafe { ffi::SDL_SetWindowTitle(self.raw.as_ptr(), c_title.as_ptr()) };
        if result {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe { ffi::SDL_DestroyWindow(self.raw.as_ptr()) };
    }
}

// SAFETY: SDL_Window は Mutex<VideoPlayerInner> 内に保持し排他アクセスを保証しているため、
// 別スレッドへの移動は安全。
unsafe impl Send for Window {}
