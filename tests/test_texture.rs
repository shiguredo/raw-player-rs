//! Texture 公開 API の単体テスト。
//!
//! `Texture::new` の寸法契約と、`update_*` の長さ・pitch・format・奇偶検証を
//! 1 つの `#[test]` にまとめ、明示 drop 後に `quit()` する。

mod common;

use raw_player::{Error, Renderer, Texture, VideoFormat, Window, quit};

use common::acquire_sdl;

/// `width * 4` の i32 溢れを防ぐ上限（`texture.rs` と同値）。
const MAX_DIMENSION: i32 = i32::MAX / 4;

fn assert_invalid_argument(err: Error) {
    match err {
        Error::InvalidArgument(_) => {}
        other => panic!("InvalidArgument を期待したが得た: {other}"),
    }
}

fn assert_invalid_argument_message(err: Error, expected: &str) {
    match &err {
        Error::InvalidArgument(_) => {
            assert_eq!(
                err.message(),
                expected,
                "InvalidArgument の message が一致しない"
            );
        }
        other => panic!("InvalidArgument を期待したが得た: {other}"),
    }
}

#[test]
fn texture_new_and_update_contracts() {
    let _guard = acquire_sdl();

    // quit() より前に Window / Renderer / Texture を確実に破棄する
    {
        let window = Window::new("texture-test", 64, 64).expect("Window::new に失敗した");
        let renderer = Renderer::new(&window).expect("Renderer::new に失敗した");

        // --- Texture::new / new_yuv の寸法検証 ---
        // Texture は Debug 未実装のため expect_err は使えない
        for (w, h) in [(0, 1), (1, 0), (-1, 16), (16, -1), (0, 0), (i32::MIN, 16)] {
            let Err(err) = Texture::new(&renderer, VideoFormat::Rgba, w, h) else {
                panic!("非正寸法は Err であるべき: {w}x{h}");
            };
            assert_invalid_argument_message(err, "width and height must be positive");
        }
        let Err(err) = Texture::new_yuv(&renderer, 0, 16) else {
            panic!("new_yuv の非正は Err であるべき");
        };
        assert_invalid_argument_message(err, "width and height must be positive");

        let too_wide = MAX_DIMENSION + 1;
        let Err(err) = Texture::new(&renderer, VideoFormat::Rgba, too_wide, 16) else {
            panic!("過大幅は Err であるべき");
        };
        assert_invalid_argument_message(
            err,
            &format!("dimensions too large: {too_wide}x16 (max {MAX_DIMENSION})"),
        );
        let Err(err) = Texture::new(&renderer, VideoFormat::Rgba, 16, too_wide) else {
            panic!("過大高さは Err であるべき");
        };
        assert_invalid_argument_message(
            err,
            &format!("dimensions too large: 16x{too_wide} (max {MAX_DIMENSION})"),
        );

        let rgba = Texture::new(&renderer, VideoFormat::Rgba, 16, 16).expect("RGBA 16x16");
        assert_eq!(rgba.width(), 16);
        assert_eq!(rgba.height(), 16);
        assert_eq!(rgba.format(), VideoFormat::Rgba);
        drop(rgba);

        let i420_ok = Texture::new(&renderer, VideoFormat::I420, 16, 16).expect("I420 16x16");
        assert_eq!(i420_ok.width(), 16);
        assert_eq!(i420_ok.height(), 16);
        assert_eq!(i420_ok.format(), VideoFormat::I420);
        drop(i420_ok);

        let yuv = Texture::new_yuv(&renderer, 16, 16).expect("new_yuv 16x16");
        assert_eq!(yuv.width(), 16);
        assert_eq!(yuv.height(), 16);
        assert_eq!(yuv.format(), VideoFormat::I420);
        drop(yuv);

        // --- update_yuv: 長さ不足 / 最小長 / 余白 ---
        {
            let mut tex = Texture::new(&renderer, VideoFormat::I420, 16, 16).expect("Texture::new");
            let y_pitch = 16;
            let uv_pitch = 8;
            let y_min = (y_pitch * 16) as usize;
            let uv_min = (uv_pitch * 8) as usize;

            let y_short = vec![0u8; y_min - 1];
            let u = vec![128u8; uv_min];
            let v = vec![128u8; uv_min];
            assert_invalid_argument(
                tex.update_yuv(&y_short, y_pitch, &u, uv_pitch, &v, uv_pitch)
                    .expect_err("Y 不足は Err"),
            );

            let y = vec![0u8; y_min];
            tex.update_yuv(&y, y_pitch, &u, uv_pitch, &v, uv_pitch)
                .expect("最小長ちょうどは Ok");

            let y_extra = vec![0u8; y_min + 8];
            let u_extra = vec![128u8; uv_min + 4];
            let v_extra = vec![128u8; uv_min + 4];
            tex.update_yuv(&y_extra, y_pitch, &u_extra, uv_pitch, &v_extra, uv_pitch)
                .expect("余白ありは Ok");
            drop(tex);
        }

        // --- update_nv12 ---
        {
            let mut tex = Texture::new(&renderer, VideoFormat::NV12, 16, 16).expect("Texture::new");
            let y_pitch = 16;
            let uv_pitch = 16;
            let y_min = (y_pitch * 16) as usize;
            let uv_min = (uv_pitch * 8) as usize;

            let y = vec![0u8; y_min];
            let uv_short = vec![0u8; uv_min - 1];
            assert_invalid_argument(
                tex.update_nv12(&y, y_pitch, &uv_short, uv_pitch)
                    .expect_err("UV 不足は Err"),
            );

            let uv = vec![0u8; uv_min];
            tex.update_nv12(&y, y_pitch, &uv, uv_pitch)
                .expect("最小長ちょうどは Ok");

            let y_extra = vec![0u8; y_min + 16];
            let uv_extra = vec![0u8; uv_min + 8];
            tex.update_nv12(&y_extra, y_pitch, &uv_extra, uv_pitch)
                .expect("余白ありは Ok");
            drop(tex);
        }

        // --- update_packed (YUY2) ---
        {
            let mut tex = Texture::new(&renderer, VideoFormat::YUY2, 16, 8).expect("Texture::new");
            let pitch = 32; // width * 2
            let min_len = (pitch * 8) as usize;

            let short = vec![0u8; min_len - 1];
            assert_invalid_argument(
                tex.update_packed(&short, pitch)
                    .expect_err("長さ不足は Err"),
            );

            let exact = vec![0u8; min_len];
            tex.update_packed(&exact, pitch)
                .expect("最小長ちょうどは Ok");

            let extra = vec![0u8; min_len + 32];
            tex.update_packed(&extra, pitch).expect("余白ありは Ok");
            drop(tex);
        }

        // --- 負 pitch / 過小 pitch ---
        {
            let mut i420 = Texture::new(&renderer, VideoFormat::I420, 16, 16).expect("I420");
            let y = vec![0u8; 16 * 16];
            let u = vec![128u8; 8 * 8];
            let v = vec![128u8; 8 * 8];
            assert_invalid_argument(
                i420.update_yuv(&y, -1, &u, 8, &v, 8)
                    .expect_err("負 y_pitch は Err"),
            );
            assert_invalid_argument(
                i420.update_yuv(&y, 16, &u, 7, &v, 8)
                    .expect_err("小さすぎる u_pitch は Err"),
            );
            drop(i420);

            let mut nv12 = Texture::new(&renderer, VideoFormat::NV12, 16, 16).expect("NV12");
            let uv = vec![0u8; 16 * 8];
            assert_invalid_argument(
                nv12.update_nv12(&y, 16, &uv, 15)
                    .expect_err("小さすぎる uv_pitch は Err"),
            );
            drop(nv12);

            let mut yuy2 = Texture::new(&renderer, VideoFormat::YUY2, 16, 8).expect("YUY2");
            let packed = vec![0u8; 32 * 8];
            assert_invalid_argument(
                yuy2.update_packed(&packed, 31)
                    .expect_err("小さすぎる pitch は Err"),
            );
            drop(yuy2);
        }

        // --- フォーマット不一致 ---
        {
            let mut i420 = Texture::new(&renderer, VideoFormat::I420, 16, 16).expect("I420");
            let y = vec![0u8; 16 * 16];
            let uv = vec![0u8; 16 * 8];
            let packed = vec![0u8; 32 * 16];
            let u = vec![128u8; 8 * 8];
            let v = vec![128u8; 8 * 8];
            assert_invalid_argument(
                i420.update_nv12(&y, 16, &uv, 16)
                    .expect_err("I420 へ update_nv12 は Err"),
            );
            assert_invalid_argument(
                i420.update_packed(&packed, 32)
                    .expect_err("I420 へ update_packed は Err"),
            );
            drop(i420);

            let mut nv12 = Texture::new(&renderer, VideoFormat::NV12, 16, 16).expect("NV12");
            assert_invalid_argument(
                nv12.update_yuv(&y, 16, &u, 8, &v, 8)
                    .expect_err("NV12 へ update_yuv は Err"),
            );
            assert_invalid_argument(
                nv12.update_packed(&packed, 32)
                    .expect_err("NV12 へ update_packed は Err"),
            );
            drop(nv12);

            let mut yuy2 = Texture::new(&renderer, VideoFormat::YUY2, 16, 8).expect("YUY2");
            assert_invalid_argument(
                yuy2.update_yuv(&y, 16, &u, 8, &v, 8)
                    .expect_err("YUY2 へ update_yuv は Err"),
            );
            drop(yuy2);
        }

        // --- 奇数寸法は作成可能だが update で拒否 ---
        {
            let mut tex = Texture::new(&renderer, VideoFormat::I420, 15, 16).expect("奇数幅 I420");
            let y = vec![0u8; 15 * 16];
            let u = vec![128u8; 8 * 8];
            let v = vec![128u8; 8 * 8];
            assert_invalid_argument(
                tex.update_yuv(&y, 15, &u, 7, &v, 7)
                    .expect_err("奇数幅 I420 の update_yuv は Err"),
            );
            drop(tex);

            let mut tex =
                Texture::new(&renderer, VideoFormat::NV12, 16, 15).expect("奇数高さ NV12");
            let y = vec![0u8; 16 * 15];
            let uv = vec![0u8; 16 * 7];
            assert_invalid_argument(
                tex.update_nv12(&y, 16, &uv, 16)
                    .expect_err("奇数高さ NV12 の update_nv12 は Err"),
            );
            drop(tex);
        }

        // --- RGBA packed 最小長 ---
        {
            let mut tex = Texture::new(&renderer, VideoFormat::Rgba, 8, 4).expect("RGBA");
            let pitch = 32; // width * 4
            let min_len = (pitch * 4) as usize;
            assert_invalid_argument(
                tex.update_packed(&vec![0u8; min_len - 1], pitch)
                    .expect_err("RGBA 長さ不足は Err"),
            );
            tex.update_packed(&vec![0u8; min_len], pitch)
                .expect("RGBA 最小長は Ok");
            drop(tex);
        }

        drop(renderer);
        drop(window);
    }

    // Safety: 上記ブロックで SDL オブジェクトを破棄済み。
    unsafe {
        quit();
    }
}
