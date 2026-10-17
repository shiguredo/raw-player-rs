//! 映像フレーム検証関数のプロパティテスト。
//!
//! 正しいサイズのフレームが受理され、壊れたサイズ・奇数の寸法・非正の寸法が
//! 拒否されることを、noprop で生成した入力に対して確認する。

use noprop::TestCaseContext;
use raw_player::video_player::{
    validate_bgra, validate_i420, validate_nv12, validate_rgba, validate_yuy2,
};

/// 1 ケースあたりの試行回数。
const CASES: usize = 256;

/// シードを取得してプロパティを実行する。
fn run<F>(property: F) -> noprop::TestResult
where
    F: Fn(&mut TestCaseContext) -> noprop::TestResult,
{
    let seed = noprop::seed_from_env_or_time("RAW_PLAYER_PBT_SEED")?;
    noprop::Runner::new(seed).run(CASES, property)?;
    Ok(())
}

/// 2 以上 1024 以下の偶数をサンプリングする。
fn sample_even_dim(ctx: &mut TestCaseContext) -> i32 {
    // 1..=512 を 2 倍し、必ず偶数にする。
    noprop::sample_i32_in(ctx, 1..=512) * 2
}

/// 1 以上 1024 以下の正の寸法をサンプリングする。
fn sample_positive_dim(ctx: &mut TestCaseContext) -> i32 {
    noprop::sample_i32_in(ctx, 1..=1024)
}

/// 0 以上 255 バイト以下の任意のバイト列をサンプリングする。
fn sample_data(ctx: &mut TestCaseContext) -> Vec<u8> {
    let len = noprop::sample_usize_in(ctx, 0..256);
    noprop::sample_bytes_vec(ctx, len)
}

/// I420 の正しいプレーンサイズが受理されること。
#[test]
fn valid_i420_accepts_correct_size() -> noprop::TestResult {
    run(|ctx| {
        let w = sample_even_dim(ctx);
        let h = sample_even_dim(ctx);
        let y = vec![0u8; (w * h) as usize];
        let uv_size = ((w / 2) * (h / 2)) as usize;
        let u = vec![0u8; uv_size];
        let v = vec![0u8; uv_size];
        assert!(
            validate_i420(&y, &u, &v, w, h).is_ok(),
            "I420 の正しいサイズが拒否された: {w}x{h}"
        );
        Ok(())
    })
}

/// I420 が奇数の幅を拒否すること。
#[test]
fn i420_rejects_odd_width() -> noprop::TestResult {
    run(|ctx| {
        let h = sample_even_dim(ctx);
        let w = 3; // 奇数
        let y = vec![0u8; (w * h) as usize];
        let u = vec![0u8; 1];
        let v = vec![0u8; 1];
        assert!(
            validate_i420(&y, &u, &v, w, h).is_err(),
            "I420 が奇数の幅 {w} を受理した (高さ {h})"
        );
        Ok(())
    })
}

/// I420 が奇数の高さを拒否すること。
#[test]
fn i420_rejects_odd_height() -> noprop::TestResult {
    run(|ctx| {
        let w = sample_even_dim(ctx);
        let h = 3; // 奇数
        let y = vec![0u8; (w * h) as usize];
        let u = vec![0u8; 1];
        let v = vec![0u8; 1];
        assert!(
            validate_i420(&y, &u, &v, w, h).is_err(),
            "I420 が奇数の高さ {h} を受理した (幅 {w})"
        );
        Ok(())
    })
}

/// I420 が Y プレーンのサイズ不一致を拒否すること。
#[test]
fn i420_rejects_wrong_y_size() -> noprop::TestResult {
    run(|ctx| {
        let w = sample_even_dim(ctx);
        let h = sample_even_dim(ctx);
        let y = vec![0u8; (w * h) as usize + 1]; // 1 バイト多い
        let uv_size = ((w / 2) * (h / 2)) as usize;
        let u = vec![0u8; uv_size];
        let v = vec![0u8; uv_size];
        assert!(
            validate_i420(&y, &u, &v, w, h).is_err(),
            "I420 が 1 バイト多い Y プレーンを受理した: {w}x{h}"
        );
        Ok(())
    })
}

/// NV12 の正しいプレーンサイズが受理されること。
#[test]
fn valid_nv12_accepts_correct_size() -> noprop::TestResult {
    run(|ctx| {
        let w = sample_even_dim(ctx);
        let h = sample_even_dim(ctx);
        let y = vec![0u8; (w * h) as usize];
        let uv = vec![0u8; (w * (h / 2)) as usize];
        assert!(
            validate_nv12(&y, &uv, w, h).is_ok(),
            "NV12 の正しいサイズが拒否された: {w}x{h}"
        );
        Ok(())
    })
}

/// NV12 が奇数の寸法を拒否すること。
#[test]
fn nv12_rejects_odd_dimensions() -> noprop::TestResult {
    run(|ctx| {
        let w = sample_even_dim(ctx);
        let h = 5; // 奇数
        let y = vec![0u8; (w * h) as usize];
        let uv = vec![0u8; 1];
        assert!(
            validate_nv12(&y, &uv, w, h).is_err(),
            "NV12 が奇数の高さ {h} を受理した (幅 {w})"
        );
        Ok(())
    })
}

/// NV12 が UV プレーンのサイズ不一致を拒否すること。
#[test]
fn nv12_rejects_wrong_uv_size() -> noprop::TestResult {
    run(|ctx| {
        let w = sample_even_dim(ctx);
        let h = sample_even_dim(ctx);
        let y = vec![0u8; (w * h) as usize];
        let uv = vec![0u8; (w * (h / 2)) as usize + 1]; // 1 バイト多い
        assert!(
            validate_nv12(&y, &uv, w, h).is_err(),
            "NV12 が 1 バイト多い UV プレーンを受理した: {w}x{h}"
        );
        Ok(())
    })
}

/// YUY2 の正しいサイズが受理されること。
#[test]
fn valid_yuy2_accepts_correct_size() -> noprop::TestResult {
    run(|ctx| {
        let w = sample_even_dim(ctx);
        let h = sample_positive_dim(ctx);
        let data = vec![0u8; (w * h * 2) as usize];
        assert!(
            validate_yuy2(&data, w, h).is_ok(),
            "YUY2 の正しいサイズが拒否された: {w}x{h}"
        );
        Ok(())
    })
}

/// YUY2 が奇数の幅を拒否すること。
#[test]
fn yuy2_rejects_odd_width() -> noprop::TestResult {
    run(|ctx| {
        let h = sample_positive_dim(ctx);
        let w = 3; // 奇数
        let data = vec![0u8; (w * h * 2) as usize];
        assert!(
            validate_yuy2(&data, w, h).is_err(),
            "YUY2 が奇数の幅 {w} を受理した (高さ {h})"
        );
        Ok(())
    })
}

/// YUY2 がサイズ不一致を拒否すること。
#[test]
fn yuy2_rejects_wrong_size() -> noprop::TestResult {
    run(|ctx| {
        let w = sample_even_dim(ctx);
        let h = sample_positive_dim(ctx);
        let data = vec![0u8; (w * h * 2) as usize + 1]; // 1 バイト多い
        assert!(
            validate_yuy2(&data, w, h).is_err(),
            "YUY2 が 1 バイト多いデータを受理した: {w}x{h}"
        );
        Ok(())
    })
}

/// RGBA の正しいサイズが受理されること。
#[test]
fn valid_rgba_accepts_correct_size() -> noprop::TestResult {
    run(|ctx| {
        let w = sample_positive_dim(ctx);
        let h = sample_positive_dim(ctx);
        let data = vec![0u8; (w * h * 4) as usize];
        assert!(
            validate_rgba(&data, w, h).is_ok(),
            "RGBA の正しいサイズが拒否された: {w}x{h}"
        );
        Ok(())
    })
}

/// RGBA がサイズ不一致を拒否すること。
#[test]
fn rgba_rejects_wrong_size() -> noprop::TestResult {
    run(|ctx| {
        let w = sample_positive_dim(ctx);
        let h = sample_positive_dim(ctx);
        let data = vec![0u8; (w * h * 4) as usize + 1]; // 1 バイト多い
        assert!(
            validate_rgba(&data, w, h).is_err(),
            "RGBA が 1 バイト多いデータを受理した: {w}x{h}"
        );
        Ok(())
    })
}

/// すべての検証関数が 0 の寸法を拒否すること。
#[test]
fn all_validators_reject_zero_dimensions() -> noprop::TestResult {
    run(|ctx| {
        let data = sample_data(ctx);
        assert!(
            validate_i420(&data, &data, &data, 0, 4).is_err(),
            "I420 が幅 0 を受理した"
        );
        assert!(
            validate_i420(&data, &data, &data, 4, 0).is_err(),
            "I420 が高さ 0 を受理した"
        );
        assert!(
            validate_nv12(&data, &data, 0, 4).is_err(),
            "NV12 が幅 0 を受理した"
        );
        assert!(
            validate_nv12(&data, &data, 4, 0).is_err(),
            "NV12 が高さ 0 を受理した"
        );
        assert!(
            validate_yuy2(&data, 0, 4).is_err(),
            "YUY2 が幅 0 を受理した"
        );
        assert!(
            validate_yuy2(&data, 4, 0).is_err(),
            "YUY2 が高さ 0 を受理した"
        );
        assert!(
            validate_rgba(&data, 0, 4).is_err(),
            "RGBA が幅 0 を受理した"
        );
        assert!(
            validate_rgba(&data, 4, 0).is_err(),
            "RGBA が高さ 0 を受理した"
        );
        Ok(())
    })
}

/// すべての検証関数が負の寸法を拒否すること。
#[test]
fn all_validators_reject_negative_dimensions() -> noprop::TestResult {
    run(|ctx| {
        let data = sample_data(ctx);
        assert!(
            validate_i420(&data, &data, &data, -2, 4).is_err(),
            "I420 が負の幅を受理した"
        );
        assert!(
            validate_nv12(&data, &data, -2, 4).is_err(),
            "NV12 が負の幅を受理した"
        );
        assert!(
            validate_yuy2(&data, -2, 4).is_err(),
            "YUY2 が負の幅を受理した"
        );
        assert!(
            validate_rgba(&data, -2, 4).is_err(),
            "RGBA が負の幅を受理した"
        );
        assert!(
            validate_bgra(&data, -2, 4).is_err(),
            "BGRA が負の幅を受理した"
        );
        Ok(())
    })
}

/// BGRA の正しいサイズが受理されること。
#[test]
fn valid_bgra_accepts_correct_size() -> noprop::TestResult {
    run(|ctx| {
        let w = sample_positive_dim(ctx);
        let h = sample_positive_dim(ctx);
        let data = vec![0u8; (w * h * 4) as usize];
        assert!(
            validate_bgra(&data, w, h).is_ok(),
            "BGRA の正しいサイズが拒否された: {w}x{h}"
        );
        Ok(())
    })
}

/// BGRA がサイズ不一致を拒否すること。
#[test]
fn bgra_rejects_wrong_size() -> noprop::TestResult {
    run(|ctx| {
        let w = sample_positive_dim(ctx);
        let h = sample_positive_dim(ctx);
        let data = vec![0u8; (w * h * 4) as usize + 1]; // 1 バイト多い
        assert!(
            validate_bgra(&data, w, h).is_err(),
            "BGRA が 1 バイト多いデータを受理した: {w}x{h}"
        );
        Ok(())
    })
}

/// RGBA と BGRA のサイズ検証結果が一致すること。
#[test]
fn bgra_and_rgba_agree() -> noprop::TestResult {
    run(|ctx| {
        let w = sample_positive_dim(ctx);
        let h = sample_positive_dim(ctx);
        let correct = vec![0u8; (w * h * 4) as usize];
        let wrong = vec![0u8; (w * h * 4) as usize + 1];
        // 正しいサイズでは両方受理される。
        assert!(
            validate_rgba(&correct, w, h).is_ok(),
            "RGBA が正しいサイズを拒否した: {w}x{h}"
        );
        assert!(
            validate_bgra(&correct, w, h).is_ok(),
            "BGRA が正しいサイズを拒否した: {w}x{h}"
        );
        // 不正なサイズでは両方拒否される。
        assert!(
            validate_rgba(&wrong, w, h).is_err(),
            "RGBA が不正なサイズを受理した: {w}x{h}"
        );
        assert!(
            validate_bgra(&wrong, w, h).is_err(),
            "BGRA が不正なサイズを受理した: {w}x{h}"
        );
        Ok(())
    })
}
