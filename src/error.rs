use crate::ffi;
use std::ffi::CStr;

#[derive(Debug, Clone)]
pub enum Error {
    Sdl(String),
    InvalidArgument(String),
    /// 再生が停止中のためデータを受け付けられない
    NotPlaying,
}

impl Error {
    pub fn from_sdl() -> Self {
        let message = unsafe {
            let ptr = ffi::SDL_GetError();
            if ptr.is_null() {
                "Unknown SDL error".to_string()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        };
        Self::Sdl(message)
    }

    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self::InvalidArgument(msg.into())
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Sdl(msg) | Self::InvalidArgument(msg) => msg,
            Self::NotPlaying => "player is not playing",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sdl(msg) => write!(f, "SDL error: {msg}"),
            Self::InvalidArgument(msg) => write!(f, "Invalid argument: {msg}"),
            Self::NotPlaying => write!(f, "Player is not playing"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
