//! VideoPlayer の公開 API に対する単体テスト。
//!
//! 映像のみ経路の pause / play で壁時計 skew が同期ドロップを起こさないことと、
//! 再生時計が PTS からずれて描画が止まったときに最新フレームへ復帰することを検証する。

#[path = "helpers/sdl.rs"]
mod helpers;

use std::thread;
use std::time::{Duration, Instant};

use raw_player::audio_format::AudioFormat;
use raw_player::error::Error;
use raw_player::video_format::VideoFormat;
use raw_player::video_player::{VideoPlayer, validate_pixel_buffer_enqueue};

use helpers::{SdlTestGuard, acquire_sdl};

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

/// 映像のみ経路の復帰テスト向けに、vsync を無効化したプレイヤーを用意する。
///
/// 映像キュー上限は既定の 5 のままにする。報告された再現条件 (キュー上限超過で古い
/// フレームが捨てられ、表示時刻に近いフレームが失われる) をそのまま再現するためである。
fn new_video_only_player(title: &str) -> VideoPlayer {
    let player = VideoPlayer::new(64, 64, title).expect("VideoPlayer::new に失敗した");
    player.set_vsync(0).expect("set_vsync(0) に失敗した");
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

/// 30 fps 相当のペースで指定回数だけ映像を投入して poll_events する。
///
/// CI の実行速度に依存しないよう、実時間の期限ではなく回数で終了を決める。`sleep` を
/// 投入の前に行い、PTS は投入の直前に採取する。これにより poll した時点の再生時計との
/// 差はほぼ 0 になり、同期しきい値 (40 ms) に余裕を持って収まる。
fn drive_normal_video(player: &VideoPlayer, start: Instant, iterations: usize) {
    for _ in 0..iterations {
        thread::sleep(Duration::from_millis(60));
        let pts_us = start.elapsed().as_micros() as i64;
        enqueue_black_i420(player, 16, 16, pts_us);
        player.poll_events().expect("poll_events に失敗した");
    }
}

/// 最新フレームへの復帰が観測されるまで、映像を投入して poll_events を繰り返す。
///
/// 復帰は「描画できない状態が 1 秒継続」で発動するため、実時間で 1 秒以上待つ必要が
/// ある。CI の実行速度に依存しないよう期限は十分に長くとり、復帰と描画の再開を観測
/// した時点で終了する。`next_pts` は投入するフレームの PTS を返す。
fn drive_until_resync(player: &VideoPlayer, mut next_pts: impl FnMut() -> i64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(20);
    for _ in 0..400 {
        enqueue_black_i420(player, 16, 16, next_pts());
        player.poll_events().expect("poll_events に失敗した");
        let stats = player.stats();
        if stats.resync_count >= 1 && stats.total_frames_rendered >= 2 {
            return true;
        }
        if Instant::now() >= deadline {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
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

/// 映像のみ再生で再生時計が PTS より遅れたとき、最新フレームへ復帰して描画が継続することを確認する。
///
/// 報告された再現手順 (PTS に試験開始からの経過時間を使い、最初のフレーム投入から
/// 最初の poll_events までだけ 300 ms 待つ) をそのまま再現する。この条件では、
/// 最初の描画で時計が PTS 0 に対応付けられたあと、以降に投入する PTS が
/// 経過時間に追随するため、時計が PTS より約 300 ms 遅れたまま固定される。
/// キュー上限 5 枚を超えた古いフレームは enqueue 時に捨てられるため、
/// 復帰しない場合は最初の 1 枚以降が永久に描画されない。
#[test]
fn video_only_resyncs_when_clock_lags_pts() {
    let _guard = acquire_sdl();
    let player = new_video_only_player("test-resync-clock-lag");

    let start = Instant::now();

    // 最初の 1 枚だけは PTS 0 で投入する
    enqueue_black_i420(&player, 16, 16, 0);

    // 最初の poll_events までだけ 300 ms 待つ。この間にアプリ側の時計だけが進む
    thread::sleep(Duration::from_millis(300));

    player.play().expect("play に失敗した");
    player.poll_events().expect("最初の poll_events に失敗した");
    assert_eq!(
        player.stats().total_frames_rendered,
        1,
        "最初の 1 枚が描画されなかった"
    );

    // PTS は試験開始からの経過時間を使う。時計は PTS より約 300 ms 遅れたままになり、
    // 復帰しない限り描画が再開しない
    let resynced = drive_until_resync(&player, || start.elapsed().as_micros() as i64);

    let stats = player.stats();
    assert!(
        resynced,
        "時計が PTS より遅れたときに描画が再開しなかった: rendered={}, resync={}",
        stats.total_frames_rendered, stats.resync_count
    );
    // 復帰後は描画が継続する
    assert!(
        stats.total_frames_rendered >= 2,
        "復帰後に描画が継続しなかった: rendered={}",
        stats.total_frames_rendered
    );
    // 時計が PTS より遅れた状態からは、最新フレームへの復帰が 1 回以上発動する
    assert!(
        stats.resync_count >= 1,
        "最新フレームへの復帰が発動しなかった: resync={}",
        stats.resync_count
    );
    assert_eq!(
        stats.dropped_frames,
        stats.dropped_frames_queue_full + stats.dropped_frames_late + stats.dropped_frames_resync,
        "dropped_frames が理由別カウンタの合計になっていない: total={}, queue_full={}, late={}, resync={}",
        stats.dropped_frames,
        stats.dropped_frames_queue_full,
        stats.dropped_frames_late,
        stats.dropped_frames_resync
    );
}

/// 映像のみ再生で再生時計が PTS より進んだとき、最新フレームへ復帰して描画が継続することを確認する。
///
/// 最初の 1 枚を描画して時計を開始したあと 300 ms 何も投入せずに待つと、
/// 時計だけが 300 ms 進む。そのあと PTS を時計より 300 ms 遅らせたまま投入し続ける
/// ことで、アプリ側の処理停滞で時計が先行した状況を再現する。
/// 復帰しない場合は全フレームが遅延ドロップされ続けて描画が止まる。
#[test]
fn video_only_resyncs_when_clock_ahead_of_pts() {
    let _guard = acquire_sdl();
    let player = new_video_only_player("test-resync-clock-ahead");

    let start = Instant::now();

    // 最初のフレームを PTS 0 で投入し、play と poll_events で 1 枚描画させて時計を開始する
    enqueue_black_i420(&player, 16, 16, 0);
    player.play().expect("play に失敗した");
    player.poll_events().expect("最初の poll_events に失敗した");
    assert_eq!(
        player.stats().total_frames_rendered,
        1,
        "最初の 1 枚が描画されなかった"
    );

    // 300 ms 何も投入せずに待ち、時計だけを進める
    thread::sleep(Duration::from_millis(300));

    // PTS は時計 (試験開始からの経過時間) より 300 ms 遅れた値のまま進める
    let resynced = drive_until_resync(&player, || start.elapsed().as_micros() as i64 - 300_000);

    let stats = player.stats();
    assert!(
        resynced,
        "時計が PTS より進んだときに描画が再開しなかった: rendered={}, resync={}",
        stats.total_frames_rendered, stats.resync_count
    );
    // 復帰しない場合は全フレームが遅延ドロップされ続けて描画が止まる
    assert!(
        stats.total_frames_rendered >= 2,
        "復帰後に描画が継続しなかった: rendered={}",
        stats.total_frames_rendered
    );
    // 時計が先行する状態では、まず遅延ドロップが発生してから復帰する
    assert!(
        stats.dropped_frames_late > 0,
        "遅延ドロップが記録されなかった: late={}",
        stats.dropped_frames_late
    );
    assert!(
        stats.resync_count >= 1,
        "最新フレームへの復帰が発動しなかった: resync={}",
        stats.resync_count
    );
    assert_eq!(
        stats.dropped_frames,
        stats.dropped_frames_queue_full + stats.dropped_frames_late + stats.dropped_frames_resync,
        "dropped_frames が理由別カウンタの合計になっていない: total={}, queue_full={}, late={}, resync={}",
        stats.dropped_frames,
        stats.dropped_frames_queue_full,
        stats.dropped_frames_late,
        stats.dropped_frames_resync
    );
}

/// 正常な 30 fps 相当の映像のみ再生では復帰が発動しないことを確認する。
///
/// PTS を投入の直前に採取した経過時間に合わせるため、時計と PTS の差は同期しきい値の
/// 内側に収まり続ける。復帰判定の 1 秒を超える長さ (20 回 × 60 ms) 実行しても
/// 復帰回数が 0 のままであることを検証する。回数で終了を決めるため CI の速度に依存しない。
#[test]
fn video_only_normal_30fps_does_not_resync() {
    let _guard = acquire_sdl();
    let player = new_video_only_player("test-no-resync");

    let start = Instant::now();

    // 最初のフレームを PTS 0 で投入し、play と poll_events で 1 枚描画させて時計を開始する
    enqueue_black_i420(&player, 16, 16, 0);
    player.play().expect("play に失敗した");
    player.poll_events().expect("最初の poll_events に失敗した");
    assert_eq!(
        player.stats().total_frames_rendered,
        1,
        "最初の 1 枚が描画されなかった"
    );

    // 30 fps 相当で 20 回投入と描画を続ける
    drive_normal_video(&player, start, 20);

    let stats = player.stats();
    assert!(
        stats.total_frames_rendered >= 10,
        "正常な再生で描画が継続しなかった: rendered={}",
        stats.total_frames_rendered
    );
    // 正常再生では描画できない状態が 1 秒続かないため、復帰は発動しない
    assert_eq!(
        stats.resync_count, 0,
        "正常な再生で復帰が誤発動した: resync={}",
        stats.resync_count
    );
    assert_eq!(
        stats.dropped_frames_resync, 0,
        "正常な再生で復帰によるドロップが発生した: resync_drop={}",
        stats.dropped_frames_resync
    );
    assert_eq!(
        stats.dropped_frames,
        stats.dropped_frames_queue_full + stats.dropped_frames_late + stats.dropped_frames_resync,
        "dropped_frames が理由別カウンタの合計になっていない: total={}, queue_full={}, late={}, resync={}",
        stats.dropped_frames,
        stats.dropped_frames_queue_full,
        stats.dropped_frames_late,
        stats.dropped_frames_resync
    );
}

/// drain_video が描画できない状態の計測をクリアすることを確認する。
///
/// drain_video のあと 1 秒以上待っても、クリアした状態から復帰が発動しないことを検証する。
#[test]
fn drain_video_clears_blocked_state() {
    let _guard = acquire_sdl();
    let player = new_video_only_player("test-drain-clears-blocked");

    // 最初の 1 枚を描画して時計を開始する
    enqueue_black_i420(&player, 16, 16, 0);
    player.play().expect("play に失敗した");
    player.poll_events().expect("最初の poll_events に失敗した");
    assert_eq!(
        player.stats().total_frames_rendered,
        1,
        "最初の 1 枚が描画されなかった"
    );

    // 時計より 300 ms 先の PTS を投入して poll し、描画できない状態を発生させる
    enqueue_black_i420(&player, 16, 16, 300_000);
    player.poll_events().expect("poll_events に失敗した");

    // キューと時計をリセットする。ここで描画できない状態の計測もクリアされる
    player.drain_video();

    // 復帰判定の 1 秒を超えて待つ
    thread::sleep(Duration::from_millis(1_100));

    // リセット後は最初のフレーム投入時と同じく、キュー先頭の PTS へ時計を再基準化して描画する
    enqueue_black_i420(&player, 16, 16, 500_000);
    player.poll_events().expect("poll_events に失敗した");

    let stats = player.stats();
    assert!(
        stats.total_frames_rendered >= 2,
        "drain_video 後の描画が行われなかった: rendered={}",
        stats.total_frames_rendered
    );
    assert_eq!(
        stats.resync_count, 0,
        "drain_video でクリアした状態から復帰が誤発動した: resync={}",
        stats.resync_count
    );
}

/// 再生中の play() 呼び出しが復帰タイマーをリセットしないことを確認する。
///
/// play() が再生中でも描画できない状態の計測をクリアすると、呼び出し側が毎フレーム
/// play() を呼ぶ場合に復帰が永久に発動せず、報告された描画停止が残る。
#[test]
fn play_while_playing_does_not_reset_the_resync_timer() {
    let _guard = acquire_sdl();
    let player = new_video_only_player("test-play-does-not-reset");

    // 最初の 1 枚を描画して映像のみ経路の時計を開始する
    enqueue_black_i420(&player, 16, 16, 0);
    player.play().expect("play に失敗した");
    player.poll_events().expect("最初の poll_events に失敗した");
    assert_eq!(
        player.stats().total_frames_rendered,
        1,
        "最初の 1 枚が描画されなかった"
    );

    // 時計が追いつかないほど先の PTS を投入し、描画できない状態を作る。
    // 時計が自然に追いつくと復帰を待たずに描画されてしまうため、十分に先にする
    enqueue_black_i420(&player, 16, 16, 10_000_000);
    player.poll_events().expect("poll_events に失敗した");

    // 再生中の play() を呼びながら 1 秒以上待つ。復帰タイマーがリセットされなければ
    // 最新フレームへの復帰が発動して描画が再開する
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        player.play().expect("play に失敗した");
        thread::sleep(Duration::from_millis(20));
        player.poll_events().expect("poll_events に失敗した");
        let stats = player.stats();
        if stats.resync_count >= 1 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "再生中の play() で復帰が発動しなかった: rendered={}, resync={}",
            stats.total_frames_rendered,
            stats.resync_count
        );
    }

    let stats = player.stats();
    assert!(
        stats.resync_count >= 1,
        "復帰回数が記録されていない: resync={}",
        stats.resync_count
    );
    assert!(
        stats.total_frames_rendered >= 2,
        "復帰後に描画されなかった: rendered={}",
        stats.total_frames_rendered
    );
}

/// pause と stop が描画できない状態の計測をクリアすることを確認する。
///
/// クリアしない場合、pause 中に進んだ時間や stop 前の状態で、再開直後に復帰が
/// 誤って発動する。
#[test]
fn pause_and_stop_clear_the_blocked_state() {
    let _guard = acquire_sdl();
    let player = new_video_only_player("test-pause-stop-clears-blocked");

    // 最初の 1 枚を描画して時計を開始する
    enqueue_black_i420(&player, 16, 16, 0);
    player.play().expect("play に失敗した");
    player.poll_events().expect("最初の poll_events に失敗した");
    assert_eq!(
        player.stats().total_frames_rendered,
        1,
        "最初の 1 枚が描画されなかった"
    );

    // 時計より 300 ms 先の PTS を投入して描画できない状態を作る
    enqueue_black_i420(&player, 16, 16, 300_000);
    player.poll_events().expect("poll_events に失敗した");

    // pause 中に 1 秒以上待っても、再開直後の poll で復帰が誤発動しないこと
    player.pause().expect("pause に失敗した");
    thread::sleep(Duration::from_millis(1_100));
    player.play().expect("play に失敗した");
    player.poll_events().expect("poll_events に失敗した");
    assert_eq!(
        player.stats().resync_count,
        0,
        "pause 中に進んだ時間で復帰が誤発動した: resync={}",
        player.stats().resync_count
    );

    // stop でキューと復帰状態が消える。再開後は最初の投入で時計が決まり、復帰は発動しない
    player.stop().expect("stop に失敗した");
    player.play().expect("play に失敗した");
    enqueue_black_i420(&player, 16, 16, 0);
    player.poll_events().expect("poll_events に失敗した");

    let stats = player.stats();
    assert_eq!(
        stats.total_frames_rendered, 1,
        "stop 後の最初のフレームが描画されなかった: rendered={}",
        stats.total_frames_rendered
    );
    assert_eq!(
        stats.resync_count, 0,
        "stop 後に復帰が誤発動した: resync={}",
        stats.resync_count
    );
}

/// キュー上限超過のドロップが理由別カウンタに記録されることを確認する。
#[test]
fn dropped_frames_records_queue_full_reason() {
    let _guard = acquire_sdl();
    let player = new_video_only_player("test-drop-reason");

    // キュー上限を 2 にして、3 枚目でキュー上限超過のドロップを起こす
    player.set_max_video_queue_size(2);
    for i in 0..3 {
        enqueue_black_i420(&player, 16, 16, i * 33_333);
    }

    let stats = player.stats();
    assert_eq!(
        stats.total_frames_enqueued, 3,
        "enqueue 数が想定と違う: enqueued={}",
        stats.total_frames_enqueued
    );
    assert_eq!(
        stats.video_queue_size, 2,
        "キュー上限が守られていない: queue={}",
        stats.video_queue_size
    );
    assert_eq!(
        stats.dropped_frames, 1,
        "キュー上限超過のドロップ数が想定と違う: dropped={}",
        stats.dropped_frames
    );
    assert_eq!(
        stats.dropped_frames_queue_full, 1,
        "キュー上限超過が理由別カウンタに入っていない: queue_full={}",
        stats.dropped_frames_queue_full
    );
    assert_eq!(
        stats.dropped_frames_late, 0,
        "描画していないのに遅延ドロップが記録された: late={}",
        stats.dropped_frames_late
    );
    assert_eq!(
        stats.dropped_frames_resync, 0,
        "描画していないのに復帰ドロップが記録された: resync={}",
        stats.dropped_frames_resync
    );
    assert_eq!(
        stats.dropped_frames,
        stats.dropped_frames_queue_full + stats.dropped_frames_late + stats.dropped_frames_resync,
        "dropped_frames が理由別カウンタの合計になっていない: total={}, queue_full={}, late={}, resync={}",
        stats.dropped_frames,
        stats.dropped_frames_queue_full,
        stats.dropped_frames_late,
        stats.dropped_frames_resync
    );
}

fn assert_invalid_argument(err: Error) {
    match err {
        Error::InvalidArgument(_) => {}
        other => panic!("InvalidArgument を期待したが得た: {other}"),
    }
}

/// PixelBuffer enqueue が非対応フォーマットを拒否することを確認する。
#[test]
fn pixel_buffer_enqueue_rejects_unsupported_formats() {
    for format in [VideoFormat::YUY2, VideoFormat::Rgba, VideoFormat::Bgra] {
        assert_invalid_argument(
            validate_pixel_buffer_enqueue(format, 16, 16).expect_err("非対応 format は Err"),
        );
    }
}

/// PixelBuffer enqueue が I420/NV12 の奇数寸法を拒否することを確認する。
#[test]
fn pixel_buffer_enqueue_rejects_odd_dimensions() {
    assert_invalid_argument(
        validate_pixel_buffer_enqueue(VideoFormat::I420, 15, 16).expect_err("奇数幅 I420 は Err"),
    );
    assert_invalid_argument(
        validate_pixel_buffer_enqueue(VideoFormat::I420, 16, 15).expect_err("奇数高さ I420 は Err"),
    );
    assert_invalid_argument(
        validate_pixel_buffer_enqueue(VideoFormat::NV12, 15, 16).expect_err("奇数幅 NV12 は Err"),
    );
    assert_invalid_argument(
        validate_pixel_buffer_enqueue(VideoFormat::NV12, 16, 15).expect_err("奇数高さ NV12 は Err"),
    );
}

/// PixelBuffer enqueue が偶数・正値の I420/NV12 を受け入れることを確認する。
#[test]
fn pixel_buffer_enqueue_accepts_even_i420_nv12() {
    validate_pixel_buffer_enqueue(VideoFormat::I420, 16, 16).expect("偶数 I420 は Ok");
    validate_pixel_buffer_enqueue(VideoFormat::NV12, 32, 18).expect("偶数 NV12 は Ok");
}
