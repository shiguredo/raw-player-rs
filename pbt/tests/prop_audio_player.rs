//! AudioPlayer の入力検証のプロパティテスト。
//!
//! 空データ・非正のサンプルレート・非正のチャンネル数・フレーム境界に
//! 揃わないデータが拒否され、正しいデータが受理されることを確認する。

use noprop::TestCaseContext;
use raw_player::audio_format::AudioFormat;
use raw_player::audio_player::AudioPlayer;

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

/// 対応している音声フォーマットをサンプリングする。
fn sample_audio_format(ctx: &mut TestCaseContext) -> AudioFormat {
    noprop::sample_choice(ctx, &[AudioFormat::S16, AudioFormat::F32])
}

/// 空のデータが拒否されること。
#[test]
fn rejects_empty_data() -> noprop::TestResult {
    run(|ctx| {
        let sample_rate = noprop::sample_i32_in(ctx, 1..=192000);
        let channels = noprop::sample_i32_in(ctx, 1..=8);
        let format = sample_audio_format(ctx);
        let player = AudioPlayer::new();
        assert!(
            player
                .enqueue_audio(&[], 0, sample_rate, channels, format)
                .is_err(),
            "空データを受理した (sample_rate={sample_rate}, channels={channels}, format={format:?})"
        );
        Ok(())
    })
}

/// 0 以下のサンプルレートが拒否されること。
#[test]
fn rejects_non_positive_sample_rate() -> noprop::TestResult {
    run(|ctx| {
        let sample_rate = noprop::sample_i32_in(ctx, -100..=0);
        let channels = noprop::sample_i32_in(ctx, 1..=8);
        let format = sample_audio_format(ctx);
        let player = AudioPlayer::new();
        let data = vec![0u8; 4];
        assert!(
            player
                .enqueue_audio(&data, 0, sample_rate, channels, format)
                .is_err(),
            "非正のサンプルレート {sample_rate} を受理した"
        );
        Ok(())
    })
}

/// 0 以下のチャンネル数が拒否されること。
#[test]
fn rejects_non_positive_channels() -> noprop::TestResult {
    run(|ctx| {
        let sample_rate = noprop::sample_i32_in(ctx, 1..=192000);
        let channels = noprop::sample_i32_in(ctx, -100..=0);
        let format = sample_audio_format(ctx);
        let player = AudioPlayer::new();
        let data = vec![0u8; 4];
        assert!(
            player
                .enqueue_audio(&data, 0, sample_rate, channels, format)
                .is_err(),
            "非正のチャンネル数 {channels} を受理した"
        );
        Ok(())
    })
}

/// フレームサイズの倍数でないデータが拒否されること。
#[test]
fn rejects_unaligned_data() -> noprop::TestResult {
    run(|ctx| {
        let sample_rate = noprop::sample_i32_in(ctx, 1..=192000);
        let channels = noprop::sample_i32_in(ctx, 1..=8);
        let format = sample_audio_format(ctx);
        let player = AudioPlayer::new();
        let frame_size = channels as usize * format.sample_size();
        // frame_size は 1 以上なので、frame_size + 1 バイトは必ず端数になる。
        let data = vec![0u8; frame_size + 1];
        assert!(
            player
                .enqueue_audio(&data, 0, sample_rate, channels, format)
                .is_err(),
            "フレーム境界に揃わないデータを受理した (frame_size={frame_size})"
        );
        Ok(())
    })
}

/// 正しいデータが受理されること。
#[test]
fn accepts_valid_data() -> noprop::TestResult {
    run(|ctx| {
        let sample_rate = noprop::sample_i32_in(ctx, 1..=192000);
        let channels = noprop::sample_i32_in(ctx, 1..=8);
        let format = sample_audio_format(ctx);
        let num_frames = noprop::sample_usize_in(ctx, 1..=100);
        let player = AudioPlayer::new();
        let frame_size = channels as usize * format.sample_size();
        let data = vec![0u8; frame_size * num_frames];
        assert!(
            player
                .enqueue_audio(&data, 0, sample_rate, channels, format)
                .is_ok(),
            "正しいデータを拒否した (sample_rate={sample_rate}, channels={channels}, frames={num_frames})"
        );
        Ok(())
    })
}
