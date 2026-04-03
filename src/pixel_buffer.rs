//! CVPixelBuffer のゼロコピーアクセス (macOS 専用)
//!
//! video-device-rs の `PixelBuffer::as_ptr()` から取得した `*mut c_void` を受け取り、
//! CVPixelBuffer のプレーンデータに直接アクセスする。

use std::ffi::c_void;

use crate::error::{Error, Result};

/// プレーンのバイト長 `stride * height`（`from_raw_parts` の第 2 引数用）。
/// オーバーフロー時は `Err` とし、未定義動作を避ける。
#[cfg(any(target_os = "macos", test))]
pub(crate) fn plane_buffer_len(stride: usize, height: usize) -> Result<usize> {
    stride.checked_mul(height).ok_or_else(|| {
        Error::invalid_argument("CVPixelBuffer plane byte length overflow (stride * height)")
    })
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn CFRetain(cf: *const c_void) -> *const c_void;
    fn CFRelease(cf: *const c_void);
    fn CVPixelBufferLockBaseAddress(pixel_buffer: *mut c_void, lock_flags: u64) -> i32;
    fn CVPixelBufferUnlockBaseAddress(pixel_buffer: *mut c_void, lock_flags: u64) -> i32;
    fn CVPixelBufferGetBaseAddressOfPlane(pixel_buffer: *mut c_void, plane_index: usize)
    -> *mut u8;
    fn CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer: *mut c_void, plane_index: usize) -> usize;
    fn CVPixelBufferGetHeightOfPlane(pixel_buffer: *mut c_void, plane_index: usize) -> usize;
}

/// kCVPixelBufferLock_ReadOnly
#[cfg(target_os = "macos")]
const K_CV_PIXEL_BUFFER_LOCK_READ_ONLY: u64 = 0x0000_0001;

/// CVPixelBuffer への参照。CFRetain/CFRelease で管理する。
pub struct PixelBufferRef {
    #[cfg(target_os = "macos")]
    ptr: *mut c_void,
    #[cfg(not(target_os = "macos"))]
    _phantom: std::marker::PhantomData<()>,
}

// Core Foundation の参照カウントはスレッドセーフ
unsafe impl Send for PixelBufferRef {}
unsafe impl Sync for PixelBufferRef {}

impl PixelBufferRef {
    /// `PixelBuffer::as_ptr()` から取得した生ポインタを受け取り、CFRetain する。
    ///
    /// # Safety
    ///
    /// `ptr` は有効な CVPixelBuffer へのポインタでなければならない。
    #[cfg(target_os = "macos")]
    pub unsafe fn from_ptr(ptr: *mut c_void) -> Result<Self> {
        if ptr.is_null() {
            return Err(Error::invalid_argument(
                "pixel_buffer pointer must not be null",
            ));
        }
        unsafe {
            CFRetain(ptr.cast_const());
        }
        Ok(Self { ptr })
    }

    /// macOS 以外では常にエラーを返す。
    ///
    /// # Safety
    ///
    /// `_ptr` は有効な CVPixelBuffer へのポインタでなければならない。
    #[cfg(not(target_os = "macos"))]
    pub unsafe fn from_ptr(_ptr: *mut c_void) -> Result<Self> {
        Err(Error::invalid_argument(
            "PixelBufferRef is only supported on macOS",
        ))
    }

    /// CVPixelBuffer をリードオンリーでロックし、各プレーンのデータにアクセスする。
    #[cfg(target_os = "macos")]
    pub(crate) fn lock(&self) -> Result<PixelBufferLock<'_>> {
        let status =
            unsafe { CVPixelBufferLockBaseAddress(self.ptr, K_CV_PIXEL_BUFFER_LOCK_READ_ONLY) };
        if status != 0 {
            return Err(Error::invalid_argument(format!(
                "CVPixelBufferLockBaseAddress failed: {status}"
            )));
        }
        Ok(PixelBufferLock { buffer: self })
    }

    #[cfg(not(target_os = "macos"))]
    pub(crate) fn lock(&self) -> Result<PixelBufferLock<'_>> {
        let _ = self;
        unreachable!("PixelBufferRef is only supported on macOS")
    }
}

#[cfg(target_os = "macos")]
impl Clone for PixelBufferRef {
    fn clone(&self) -> Self {
        unsafe {
            CFRetain(self.ptr.cast_const());
        }
        Self { ptr: self.ptr }
    }
}

#[cfg(target_os = "macos")]
impl Drop for PixelBufferRef {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.ptr.cast_const());
        }
    }
}

/// CVPixelBuffer のロック中にプレーンデータへのアクセスを提供する RAII ガード。
pub(crate) struct PixelBufferLock<'a> {
    #[allow(dead_code)]
    buffer: &'a PixelBufferRef,
}

impl<'a> PixelBufferLock<'a> {
    /// 指定プレーンのバイト列を返す。
    #[cfg(target_os = "macos")]
    pub fn plane(&self, index: usize) -> Result<&[u8]> {
        unsafe {
            let ptr = CVPixelBufferGetBaseAddressOfPlane(self.buffer.ptr, index);
            if ptr.is_null() {
                return Err(Error::invalid_argument(
                    "CVPixelBuffer plane base address is null",
                ));
            }
            let stride = CVPixelBufferGetBytesPerRowOfPlane(self.buffer.ptr, index);
            let height = CVPixelBufferGetHeightOfPlane(self.buffer.ptr, index);
            let len = plane_buffer_len(stride, height)?;
            Ok(std::slice::from_raw_parts(ptr, len))
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn plane(&self, _index: usize) -> Result<&[u8]> {
        Err(Error::invalid_argument(
            "PixelBufferLock is only available on macOS",
        ))
    }

    /// 指定プレーンの stride (バイト/行) を返す。
    /// SDL が要求する `i32` に安全に変換し、収まらない場合は `Err` を返す。
    #[cfg(target_os = "macos")]
    pub fn stride(&self, index: usize) -> Result<i32> {
        let stride = unsafe { CVPixelBufferGetBytesPerRowOfPlane(self.buffer.ptr, index) };
        i32::try_from(stride).map_err(|_| {
            Error::invalid_argument(format!(
                "CVPixelBuffer plane stride {stride} exceeds i32::MAX"
            ))
        })
    }

    #[cfg(not(target_os = "macos"))]
    pub fn stride(&self, _index: usize) -> Result<i32> {
        unreachable!("PixelBufferLock is only available on macOS")
    }
}

#[cfg(target_os = "macos")]
impl Drop for PixelBufferLock<'_> {
    fn drop(&mut self) {
        unsafe {
            CVPixelBufferUnlockBaseAddress(self.buffer.ptr, K_CV_PIXEL_BUFFER_LOCK_READ_ONLY);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::plane_buffer_len;

    #[test]
    fn plane_buffer_len_ok() {
        assert_eq!(plane_buffer_len(640, 480).unwrap(), 640 * 480);
    }

    #[test]
    fn plane_buffer_len_overflow_is_err() {
        assert!(plane_buffer_len(usize::MAX, 2).is_err());
    }
}
