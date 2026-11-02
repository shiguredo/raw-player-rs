//! SDL3 ウィンドウの薄いラッパー。

use crate::error::{Error, Result};
use crate::sys;
use std::ffi::CString;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::MutexGuard;

// bindgen は SDL3 の `#define` マクロ定数を生成しないため手動定義。
const SDL_WINDOW_RESIZABLE: u64 = 0x0000_0000_0000_0020;
const SDL_WINDOW_HIGH_PIXEL_DENSITY: u64 = 0x0000_0000_0000_2000;

pub struct Window {
    raw: NonNull<sys::SDL_Window>,
    // SDL_Window はスレッドアフィニティがあり、Mutex による排他だけでは Send にできない。
    _not_send: PhantomData<MutexGuard<'static, ()>>,
}

impl Window {
    pub fn new(title: &str, width: i32, height: i32) -> Result<Self> {
        let c_title =
            CString::new(title).map_err(|_| Error::invalid_argument("title contains null byte"))?;
        let flags = SDL_WINDOW_RESIZABLE | SDL_WINDOW_HIGH_PIXEL_DENSITY;
        let raw = unsafe { sys::SDL_CreateWindow(c_title.as_ptr(), width, height, flags) };
        NonNull::new(raw)
            .map(|raw| Self {
                raw,
                _not_send: PhantomData,
            })
            .ok_or_else(Error::from_sdl)
    }

    pub fn as_ptr(&self) -> *mut sys::SDL_Window {
        self.raw.as_ptr()
    }

    pub fn size(&self) -> (i32, i32) {
        let mut w = 0;
        let mut h = 0;
        unsafe { sys::SDL_GetWindowSize(self.raw.as_ptr(), &mut w, &mut h) };
        (w, h)
    }

    pub fn set_size(&mut self, width: i32, height: i32) -> Result<()> {
        let result = unsafe { sys::SDL_SetWindowSize(self.raw.as_ptr(), width, height) };
        if result {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }

    pub fn set_title(&mut self, title: &str) -> Result<()> {
        let c_title =
            CString::new(title).map_err(|_| Error::invalid_argument("title contains null byte"))?;
        let result = unsafe { sys::SDL_SetWindowTitle(self.raw.as_ptr(), c_title.as_ptr()) };
        if result {
            Ok(())
        } else {
            Err(Error::from_sdl())
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe { sys::SDL_DestroyWindow(self.raw.as_ptr()) };
    }
}
