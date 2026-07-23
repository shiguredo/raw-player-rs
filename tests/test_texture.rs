//! Texture::update_* の公開 API に対する単体テスト。
//!
//! 長さ不足・不正 pitch・フォーマット不一致で FFI 前に Err になることと、
//! 最小長ちょうど／余白ありで更新できることを検証する。

use raw_player::{Error, Renderer, Texture, VideoFormat, Window, init};

/// テスト用に SDL のダミードライバを設定し初期化する。
fn setup_sdl() {
    // Safety: テストプロセス内・SDL 初期化前のみ。
    unsafe {
        std::env::set_var("SDL_VIDEODRIVER", "dummy");
        std::env::set_var("SDL_AUDIODRIVER", "dummy");
    }
    init().expect("SDL init に失敗した");
}

fn make_renderer() -> (Window, Renderer) {
    setup_sdl();
    let window = Window::new("test-texture", 64, 64).expect("Window::new に失敗した");
    let renderer = Renderer::new(&window).expect("Renderer::new に失敗した");
    (window, renderer)
}

fn assert_invalid_argument(err: Error) {
    match err {
        Error::InvalidArgument(_) => {}
        other => panic!("InvalidArgument を期待したが得た: {other}"),
    }
}

#[test]
fn update_yuv_rejects_short_buffer_and_accepts_min_and_extra() {
    let (_window, renderer) = make_renderer();
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
}

#[test]
fn update_nv12_rejects_short_buffer_and_accepts_min_and_extra() {
    let (_window, renderer) = make_renderer();
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
}

#[test]
fn update_packed_rejects_short_buffer_and_accepts_min_and_extra() {
    let (_window, renderer) = make_renderer();
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
}

#[test]
fn update_rejects_negative_and_too_small_pitch() {
    let (_window, renderer) = make_renderer();
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

    let mut nv12 = Texture::new(&renderer, VideoFormat::NV12, 16, 16).expect("NV12");
    let uv = vec![0u8; 16 * 8];
    assert_invalid_argument(
        nv12.update_nv12(&y, 16, &uv, 15)
            .expect_err("小さすぎる uv_pitch は Err"),
    );

    let mut yuy2 = Texture::new(&renderer, VideoFormat::YUY2, 16, 8).expect("YUY2");
    let packed = vec![0u8; 32 * 8];
    assert_invalid_argument(
        yuy2.update_packed(&packed, 31)
            .expect_err("小さすぎる pitch は Err"),
    );
}

#[test]
fn update_rejects_format_mismatch() {
    let (_window, renderer) = make_renderer();
    let mut i420 = Texture::new(&renderer, VideoFormat::I420, 16, 16).expect("I420");
    let y = vec![0u8; 16 * 16];
    let uv = vec![0u8; 16 * 8];
    let packed = vec![0u8; 32 * 16];
    assert_invalid_argument(
        i420.update_nv12(&y, 16, &uv, 16)
            .expect_err("I420 へ update_nv12 は Err"),
    );
    assert_invalid_argument(
        i420.update_packed(&packed, 32)
            .expect_err("I420 へ update_packed は Err"),
    );

    let mut nv12 = Texture::new(&renderer, VideoFormat::NV12, 16, 16).expect("NV12");
    let u = vec![128u8; 8 * 8];
    let v = vec![128u8; 8 * 8];
    assert_invalid_argument(
        nv12.update_yuv(&y, 16, &u, 8, &v, 8)
            .expect_err("NV12 へ update_yuv は Err"),
    );
    assert_invalid_argument(
        nv12.update_packed(&packed, 32)
            .expect_err("NV12 へ update_packed は Err"),
    );

    let mut yuy2 = Texture::new(&renderer, VideoFormat::YUY2, 16, 8).expect("YUY2");
    assert_invalid_argument(
        yuy2.update_yuv(&y, 16, &u, 8, &v, 8)
            .expect_err("YUY2 へ update_yuv は Err"),
    );
}

#[test]
fn update_yuv_rejects_odd_dimensions_when_texture_exists() {
    let (_window, renderer) = make_renderer();
    // Texture::new が奇数寸法を拒否しない場合に update 側で守る
    let created = Texture::new(&renderer, VideoFormat::I420, 15, 16);
    let Ok(mut tex) = created else {
        // 作成時に拒否される環境では update 経路に届かない（寸法ガードは作成側と合わせて担保）
        return;
    };
    let y = vec![0u8; 15 * 16];
    let u = vec![128u8; 8 * 8];
    let v = vec![128u8; 8 * 8];
    assert_invalid_argument(
        tex.update_yuv(&y, 15, &u, 7, &v, 7)
            .expect_err("奇数幅 I420 の update_yuv は Err"),
    );
}

#[test]
fn update_nv12_rejects_odd_dimensions_when_texture_exists() {
    let (_window, renderer) = make_renderer();
    let created = Texture::new(&renderer, VideoFormat::NV12, 16, 15);
    let Ok(mut tex) = created else {
        return;
    };
    let y = vec![0u8; 16 * 15];
    let uv = vec![0u8; 16 * 7];
    assert_invalid_argument(
        tex.update_nv12(&y, 16, &uv, 16)
            .expect_err("奇数高さ NV12 の update_nv12 は Err"),
    );
}

#[test]
fn update_packed_rgba_min_length() {
    let (_window, renderer) = make_renderer();
    let mut tex = Texture::new(&renderer, VideoFormat::Rgba, 8, 4).expect("RGBA");
    let pitch = 32; // width * 4
    let min_len = (pitch * 4) as usize;
    assert_invalid_argument(
        tex.update_packed(&vec![0u8; min_len - 1], pitch)
            .expect_err("RGBA 長さ不足は Err"),
    );
    tex.update_packed(&vec![0u8; min_len], pitch)
        .expect("RGBA 最小長は Ok");
}
