use crate::pixel_buffer::PixelBufferRef;
use crate::video_format::VideoFormat;

pub(crate) enum FrameData {
    /// I420: Y / U / V の 3 プレーン
    Planar { y: Vec<u8>, u: Vec<u8>, v: Vec<u8> },
    /// NV12: Y / UV の 2 プレーン
    SemiPlanar { y: Vec<u8>, uv: Vec<u8> },
    /// YUY2 / RGBA / BGRA: インターリーブ
    Packed(Vec<u8>),
    /// CVPixelBuffer を直接保持 (macOS ゼロコピー)
    PixelBuffer(PixelBufferRef),
}

impl FrameData {
    pub(crate) fn size_bytes(&self) -> usize {
        match self {
            Self::Planar { y, u, v } => y.len() + u.len() + v.len(),
            Self::SemiPlanar { y, uv } => y.len() + uv.len(),
            Self::Packed(data) => data.len(),
            Self::PixelBuffer(_) => 0,
        }
    }
}

pub(crate) struct VideoFrame {
    pub pts_us: i64,
    pub width: i32,
    pub height: i32,
    pub y_pitch: i32,
    pub uv_pitch: i32,
    pub format: VideoFormat,
    pub data: FrameData,
}
