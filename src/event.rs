//! SDL3 のイベント取得とキーコード定数。

use crate::sys;
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
    let mut event = MaybeUninit::<sys::SDL_Event>::uninit();
    if unsafe { sys::SDL_PollEvent(event.as_mut_ptr()) } {
        let event = unsafe { event.assume_init() };
        let event_type = unsafe { event.type_ };

        // bindgen は環境によって enum 定数の基底型を i32 / u32 に分ける。
        // `as u32` は不要なキャストの警告が出る環境があるため、ビット列を経由して揃える。
        const QUIT: u32 = u32::from_ne_bytes(sys::SDL_EventType_SDL_EVENT_QUIT.to_ne_bytes());
        const KEY_DOWN: u32 =
            u32::from_ne_bytes(sys::SDL_EventType_SDL_EVENT_KEY_DOWN.to_ne_bytes());
        const KEY_UP: u32 = u32::from_ne_bytes(sys::SDL_EventType_SDL_EVENT_KEY_UP.to_ne_bytes());
        const WINDOW_RESIZED: u32 =
            u32::from_ne_bytes(sys::SDL_EventType_SDL_EVENT_WINDOW_RESIZED.to_ne_bytes());
        const WINDOW_CLOSE: u32 =
            u32::from_ne_bytes(sys::SDL_EventType_SDL_EVENT_WINDOW_CLOSE_REQUESTED.to_ne_bytes());

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
pub const KEYCODE_ESCAPE: u32 = u32::from_ne_bytes(sys::SDLK_ESCAPE.to_ne_bytes());

/// S キーのキーコード。
pub const KEYCODE_S: u32 = u32::from_ne_bytes(sys::SDLK_S.to_ne_bytes());
