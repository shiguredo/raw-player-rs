//! VideoPlayer の公開 API に対する単体テスト。
//!
//! 映像のみ経路の pause / play で壁時計 skew が同期ドロップを起こさないことを検証する。

mod common;

use std::thread;
use std::time::Duration;

use raw_player::{AudioFormat, VideoPlayer};

use common::{SdlTestGuard, acquire_sdl};

/// 指定 PTS で黒 I420 フレームを enqueue する。
fn enqueue_black_i420(player: &VideoPlayer, width: i32, height: i32, pts_us: i64) {
    let y_size = (width * height) as usize;
    let uv_size = ((width / 2) * (height / 2)) as usize;
    let y = vec![0u8; y_size];
    let u = vec![128u8; uv_size];
    let v = vec![128u8; uv_size];
    player
        .enqueue_video_i420(&y, &u, &v, width, height, pts_us)
        .expect("I420 の enqueue に失敗した");
}

/// 映像のみ経路向けにプレイヤーを用意し、指定枚数のフレームを enqueue する。
fn prepare_video_only_player(_guard: &SdlTestGuard, frame_count: usize) -> VideoPlayer {
    let player =
        VideoPlayer::new(64, 64, "test-video-only-pause").expect("VideoPlayer::new に失敗した");
    player.set_vsync(0).expect("set_vsync(0) に失敗した");
    // キュー溢れによるドロップと同期ドロップを混同しない
    player.set_max_video_queue_size(frame_count + 8);

    let width = 16;
    let height = 16;
    let pts_step_us = 33_333i64;
    for i in 0..frame_count {
        enqueue_black_i420(&player, width, height, i as i64 * pts_step_us);
    }
    player
}

/// 少なくとも 1 枚描画されるまで poll_events する。
fn poll_until_rendered(player: &VideoPlayer) {
    for _ in 0..60 {
        let open = player.poll_events().expect("poll_events に失敗した");
        assert!(open, "ウィンドウが閉じられた");
        if player.stats().total_frames_rendered >= 1 {
            return;
        }
    }
    panic!("描画が開始されなかった");
}

/// 映像のみ再生で、sync_threshold を超える pause 後の play でも同期ドロップが増えないことを確認する。
#[test]
fn video_only_pause_play_does_not_sync_drop() {
    let guard = acquire_sdl();
    let player = prepare_video_only_player(&guard, 10);
    player.play().expect("play に失敗した");
    poll_until_rendered(&player);

    // 描画直後に計測してすぐ pause する（描画〜pause 間に閾値超の実時間を入れない）
    let dropped0 = player.stats().dropped_frames;
    let q0 = player.stats().video_queue_size;
    assert!(q0 >= 1, "pause 前にキューが空: q0={q0}");
    player.pause().expect("pause に失敗した");

    thread::sleep(Duration::from_millis(200));
    player.play().expect("再開 play に失敗した");
    player
        .poll_events()
        .expect("再開後の poll_events に失敗した");

    let dropped1 = player.stats().dropped_frames;
    let q1 = player.stats().video_queue_size;
    assert_eq!(
        dropped1 - dropped0,
        0,
        "pause 後の再開で同期ドロップが増えた: before={dropped0}, after={dropped1}"
    );
    assert!(q1 > 0, "再開後にキューが空になった: q1={q1}");
}

/// 複数回の pause / play でも skew が蓄積せず、同期ドロップが増えないことを確認する。
#[test]
fn video_only_repeated_pause_play_does_not_accumulate_skew() {
    let guard = acquire_sdl();
    let player = prepare_video_only_player(&guard, 20);
    player.play().expect("play に失敗した");
    poll_until_rendered(&player);

    for round in 1..=2 {
        let dropped0 = player.stats().dropped_frames;
        let q0 = player.stats().video_queue_size;
        assert!(q0 >= 1, "round {round}: pause 前にキューが空: q0={q0}");
        player.pause().expect("pause に失敗した");

        thread::sleep(Duration::from_millis(200));
        player.play().expect("再開 play に失敗した");
        player
            .poll_events()
            .expect("再開後の poll_events に失敗した");

        let dropped1 = player.stats().dropped_frames;
        assert_eq!(
            dropped1 - dropped0,
            0,
            "round {round}: 同期ドロップが増えた: before={dropped0}, after={dropped1}"
        );
        assert!(
            player.stats().video_queue_size > 0,
            "round {round}: 再開後にキューが空になった"
        );
    }
}

/// 音声あり経路の pause / play で、本修正による同期ドロップ増が起きないことを確認する。
#[test]
fn audio_video_pause_play_does_not_worsen_sync_drops() {
    let _guard = acquire_sdl();
    let player = VideoPlayer::new(64, 64, "test-av-pause").expect("VideoPlayer::new に失敗した");
    player.set_vsync(0).expect("set_vsync(0) に失敗した");
    player.set_max_video_queue_size(24);

    let width = 16;
    let height = 16;
    let pts_step_us = 33_333i64;
    for i in 0..12 {
        enqueue_black_i420(&player, width, height, i as i64 * pts_step_us);
    }

    // 十分な長さの無音を入れ、音声ストリーム開始を確実にする
    let sample_rate = 48_000i32;
    let channels = 2i32;
    let silence = vec![0u8; sample_rate as usize * channels as usize * 2]; // 1 秒分 S16
    player
        .enqueue_audio(&silence, 0, sample_rate, channels, AudioFormat::S16)
        .expect("音声 enqueue に失敗した");

    player.play().expect("play に失敗した");

    // 音声 enqueue 後の play でストリームは開始済み。映像が 1 枚以上描画されるまで待つ
    for _ in 0..120 {
        player.poll_events().expect("poll_events に失敗した");
        if player.stats().total_frames_rendered >= 1 {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert!(
        player.stats().total_frames_rendered >= 1,
        "映像が描画されなかった"
    );

    let dropped0 = player.stats().dropped_frames;
    let q0 = player.stats().video_queue_size;
    assert!(q0 >= 1, "pause 前にキューが空: q0={q0}");
    player.pause().expect("pause に失敗した");

    thread::sleep(Duration::from_millis(200));
    player.play().expect("再開 play に失敗した");
    player
        .poll_events()
        .expect("再開後の poll_events に失敗した");

    let dropped1 = player.stats().dropped_frames;
    assert_eq!(
        dropped1 - dropped0,
        0,
        "音声あり経路で同期ドロップが増えた: before={dropped0}, after={dropped1}"
    );
}
