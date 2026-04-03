use crate::ffi;
use std::mem::MaybeUninit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Quit,
    KeyDown { keycode: u32 },
    KeyUp { keycode: u32 },
    WindowResized { width: i32, height: i32 },
    WindowClose,
    Other,
}

pub fn poll_event() -> Option<Event> {
    let mut event = MaybeUninit::<ffi::SDL_Event>::uninit();
    if unsafe { ffi::SDL_PollEvent(event.as_mut_ptr()) } {
        let event = unsafe { event.assume_init() };
        let event_type = unsafe { event.type_ };

        // bindgen は Windows (MSVC) で enum 定数を i32、Linux (GCC) で u32 として
        // 生成するため、定数を u32 にキャストして統一する
        #[allow(clippy::unnecessary_cast)]
        const QUIT: u32 = ffi::SDL_EventType_SDL_EVENT_QUIT as u32;
        #[allow(clippy::unnecessary_cast)]
        const KEY_DOWN: u32 = ffi::SDL_EventType_SDL_EVENT_KEY_DOWN as u32;
        #[allow(clippy::unnecessary_cast)]
        const KEY_UP: u32 = ffi::SDL_EventType_SDL_EVENT_KEY_UP as u32;
        #[allow(clippy::unnecessary_cast)]
        const WINDOW_RESIZED: u32 = ffi::SDL_EventType_SDL_EVENT_WINDOW_RESIZED as u32;
        #[allow(clippy::unnecessary_cast)]
        const WINDOW_CLOSE: u32 = ffi::SDL_EventType_SDL_EVENT_WINDOW_CLOSE_REQUESTED as u32;

        Some(match event_type {
            QUIT => Event::Quit,
            KEY_DOWN => {
                let key = unsafe { event.key };
                Event::KeyDown { keycode: key.key }
            }
            KEY_UP => {
                let key = unsafe { event.key };
                Event::KeyUp { keycode: key.key }
            }
            WINDOW_RESIZED => {
                let window = unsafe { event.window };
                Event::WindowResized {
                    width: window.data1,
                    height: window.data2,
                }
            }
            WINDOW_CLOSE => Event::WindowClose,
            _ => Event::Other,
        })
    } else {
        None
    }
}

/// ESC キーのキーコード。
#[allow(clippy::unnecessary_cast)]
pub const KEYCODE_ESCAPE: u32 = ffi::SDLK_ESCAPE as u32;

/// S キーのキーコード。
#[allow(clippy::unnecessary_cast)]
pub const KEYCODE_S: u32 = ffi::SDLK_S as u32;
