use crate::ffi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    S16,
    F32,
}

impl AudioFormat {
    pub fn sample_size(self) -> usize {
        match self {
            Self::S16 => 2,
            Self::F32 => 4,
        }
    }

    pub fn is_float(self) -> bool {
        matches!(self, Self::F32)
    }

    pub(crate) fn to_sdl(self) -> ffi::SDL_AudioFormat {
        match self {
            Self::S16 => ffi::SDL_AudioFormat_SDL_AUDIO_S16,
            Self::F32 => ffi::SDL_AudioFormat_SDL_AUDIO_F32,
        }
    }
}
