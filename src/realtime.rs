//! リアルタイム麻雀のロジック層。
//!
//! 「同時打牌」「時間切れの自動ツモ切り」「鳴き宣言の優先順位解決」を
//! 時間管理から切り離して純粋関数で表現する。実時間のタイマーやスレッド連携は呼び出し側
//! （CLI/wasm/web）の責務。
//!
//! # 設計指針
//! - Rust core はあくまで「状態遷移」だけを扱う。`std::time::Instant` を内部で持たない
//! - 時間進行は呼び出し側の `tick_timers(delta_ms)` 経由で注入する（テスト容易性）
//! - 鳴き宣言の優先順位（ロン > ポン > カン > チー）は `CallKind` の `Ord` 実装で表現

use crate::tile::Tile;

/// プレイヤー 1 名あたりの標準制限時間（ms）。
///
/// Issue #20 の合意値は 5 秒。`PlayerTimer::default_limit()` でも参照する。
pub const DEFAULT_TIMER_LIMIT_MS: u64 = 5000;

/// 鳴き宣言の種類（リアルタイム解決用）。
///
/// 優先順位は数値の小さい順:
/// - `Ron` (0) > `Pon` (1) > `Kan` (2) > `Chi` (3)
///
/// `derive(PartialOrd, Ord)` は列挙子の宣言順に従うため、優先順位通りに並べてある。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CallKind {
    Ron,
    Pon,
    Kan,
    Chi,
}

/// 1 件の鳴き宣言。
///
/// - `player_idx`: 宣言した席（0..=3）
/// - `kind`: 鳴きの種類
///
/// `tile`（対象の捨て牌）は `Game::last_discard` から取得できるためここでは持たない。
/// 同フレームに複数の宣言が来た場合の解決のみが本構造体の責務。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Call {
    pub player_idx: usize,
    pub kind: CallKind,
}

/// 複数の鳴き宣言から、優先順位通りに 1 つだけを採用する。
///
/// 同優先のときは入力順 (`calls` の Vec 順) で**先勝ち**。
/// 呼び出し側で「先に届いた宣言ほど先頭に入れる」キューを維持する想定。
///
/// # Example
/// ```
/// use xmj_core::realtime::{Call, CallKind, resolve_calls};
/// let calls = vec![
///     Call { player_idx: 1, kind: CallKind::Pon },
///     Call { player_idx: 2, kind: CallKind::Ron },
/// ];
/// // 後から来ても Ron が勝つ
/// assert_eq!(resolve_calls(&calls).unwrap().kind, CallKind::Ron);
/// ```
pub fn resolve_calls(calls: &[Call]) -> Option<Call> {
    // min_by_key は同値のとき**最初に出現した要素**を返す（公式 doc 保証）。
    // したがって「同優先 → 入力順で先勝ち」が自動的に成立する。
    calls.iter().copied().min_by_key(|c| c.kind)
}

/// 自動ツモ切り判定。
///
/// `elapsed_ms` が `limit_ms` 以上ならタイムアウト（true）。
/// 端ケース: `limit_ms == 0` のときは `elapsed_ms == 0` でも true（常時タイムアウト扱い）。
pub fn should_auto_discard(elapsed_ms: u64, limit_ms: u64) -> bool {
    elapsed_ms >= limit_ms
}

/// プレイヤーごとのタイマー状態。
///
/// 「直前の打牌（または初期化）からの経過 ms」と「制限時間 ms」だけを持つ。
/// 実時間進行は呼び出し側が `tick(delta_ms)` を周期的に呼ぶことで進める。
#[derive(Debug, Clone, Copy)]
pub struct PlayerTimer {
    /// 直前の打牌（または `reset`）からの経過 ms
    pub elapsed_ms: u64,
    /// 制限時間 ms（デフォルト [`DEFAULT_TIMER_LIMIT_MS`] = 5000）
    pub limit_ms: u64,
}

impl PlayerTimer {
    /// 任意の制限時間で初期化。`elapsed_ms` は 0 から開始。
    pub fn new(limit_ms: u64) -> Self {
        Self { elapsed_ms: 0, limit_ms }
    }

    /// デフォルト 5000ms で初期化。
    pub fn default_limit() -> Self {
        Self::new(DEFAULT_TIMER_LIMIT_MS)
    }

    /// 経過時間を `delta_ms` 進める。`u64::MAX` まで飽和。
    pub fn tick(&mut self, delta_ms: u64) {
        self.elapsed_ms = self.elapsed_ms.saturating_add(delta_ms);
    }

    /// 制限時間を超えているか。
    pub fn is_timeout(&self) -> bool {
        should_auto_discard(self.elapsed_ms, self.limit_ms)
    }

    /// 経過時間を 0 に戻す。`limit_ms` はそのまま。
    /// 打牌成功後に呼ぶ想定。
    pub fn reset(&mut self) {
        self.elapsed_ms = 0;
    }
}

impl Default for PlayerTimer {
    fn default() -> Self {
        Self::default_limit()
    }
}

/// 自動ツモ切り対象の牌を選ぶ補助関数。
///
/// 「手牌の末尾 = 最新ツモ牌」というプレイヤー実装の慣習に従って末尾を返す。
/// 手牌が空なら None。`Tile` のみを返し、実際の打牌処理は呼び出し側で行う。
pub fn pick_auto_discard_tile(tiles: &[Tile]) -> Option<Tile> {
    tiles.last().copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::{Suit, Tile};

    // ========================================
    // resolve_calls の優先順位
    // ========================================

    /// ロンとポンが同フレームに来たらロン勝ち（順序逆でも）。
    #[test]
    fn test_resolve_calls_prefers_ron_over_pon() {
        let calls = vec![
            Call { player_idx: 1, kind: CallKind::Pon },
            Call { player_idx: 2, kind: CallKind::Ron },
        ];
        let winner = resolve_calls(&calls).expect("at least one call resolves");
        assert_eq!(winner.kind, CallKind::Ron);
        assert_eq!(winner.player_idx, 2);
    }

    /// ポンとチーが同フレームならポン勝ち。
    #[test]
    fn test_resolve_calls_prefers_pon_over_chi() {
        let calls = vec![
            Call { player_idx: 3, kind: CallKind::Chi },
            Call { player_idx: 1, kind: CallKind::Pon },
        ];
        let winner = resolve_calls(&calls).expect("resolves");
        assert_eq!(winner.kind, CallKind::Pon);
        assert_eq!(winner.player_idx, 1);
    }

    /// カンとチーならカン勝ち（中間優先確認）。
    #[test]
    fn test_resolve_calls_prefers_kan_over_chi() {
        let calls = vec![
            Call { player_idx: 3, kind: CallKind::Chi },
            Call { player_idx: 2, kind: CallKind::Kan },
        ];
        let winner = resolve_calls(&calls).expect("resolves");
        assert_eq!(winner.kind, CallKind::Kan);
    }

    /// 空ベクタなら None。
    #[test]
    fn test_resolve_calls_returns_none_when_empty() {
        let calls: Vec<Call> = Vec::new();
        assert!(resolve_calls(&calls).is_none());
    }

    /// 同優先（同じ CallKind）のとき、入力順で**先**に来た方を採用する。
    #[test]
    fn test_resolve_calls_same_priority_first_wins() {
        let calls = vec![
            Call { player_idx: 2, kind: CallKind::Pon },
            Call { player_idx: 3, kind: CallKind::Pon },
        ];
        let winner = resolve_calls(&calls).expect("resolves");
        assert_eq!(winner.player_idx, 2, "同優先なら先入力が勝つ");
    }

    /// 3 種混在のフルセットでも Ron が勝つ。
    #[test]
    fn test_resolve_calls_full_mix_picks_ron() {
        let calls = vec![
            Call { player_idx: 3, kind: CallKind::Chi },
            Call { player_idx: 1, kind: CallKind::Pon },
            Call { player_idx: 2, kind: CallKind::Kan },
            Call { player_idx: 0, kind: CallKind::Ron },
        ];
        let winner = resolve_calls(&calls).expect("resolves");
        assert_eq!(winner.kind, CallKind::Ron);
        assert_eq!(winner.player_idx, 0);
    }

    // ========================================
    // PlayerTimer
    // ========================================

    /// tick で elapsed が加算される。
    #[test]
    fn test_player_timer_tick_accumulates() {
        let mut t = PlayerTimer::new(5000);
        assert_eq!(t.elapsed_ms, 0);
        t.tick(100);
        assert_eq!(t.elapsed_ms, 100);
        t.tick(250);
        assert_eq!(t.elapsed_ms, 350);
    }

    /// limit ちょうどでタイムアウト判定。
    #[test]
    fn test_player_timer_is_timeout_at_limit() {
        let mut t = PlayerTimer::new(5000);
        t.tick(4999);
        assert!(!t.is_timeout(), "limit 未満は false");
        t.tick(1);
        assert!(t.is_timeout(), "limit ちょうどで true");
        t.tick(10000);
        assert!(t.is_timeout(), "超過後も true");
    }

    /// reset で elapsed が 0 に戻り、limit は維持。
    #[test]
    fn test_player_timer_reset_clears_elapsed() {
        let mut t = PlayerTimer::new(5000);
        t.tick(4000);
        assert_eq!(t.elapsed_ms, 4000);
        t.reset();
        assert_eq!(t.elapsed_ms, 0);
        assert_eq!(t.limit_ms, 5000, "limit はリセットされない");
    }

    /// limit 未満では should_auto_discard が false。
    #[test]
    fn test_should_auto_discard_below_limit_false() {
        assert!(!should_auto_discard(0, 5000));
        assert!(!should_auto_discard(4999, 5000));
        assert!(should_auto_discard(5000, 5000));
        assert!(should_auto_discard(10000, 5000));
    }

    /// tick が overflow しない（saturating）。
    #[test]
    fn test_player_timer_tick_saturates_on_overflow() {
        let mut t = PlayerTimer::new(5000);
        t.elapsed_ms = u64::MAX - 10;
        t.tick(1000);
        assert_eq!(t.elapsed_ms, u64::MAX, "overflow せずに飽和する");
    }

    /// default_limit() は 5000ms。
    #[test]
    fn test_player_timer_default_limit_is_5000() {
        let t = PlayerTimer::default_limit();
        assert_eq!(t.limit_ms, DEFAULT_TIMER_LIMIT_MS);
        assert_eq!(t.limit_ms, 5000);
        assert_eq!(t.elapsed_ms, 0);
    }

    /// pick_auto_discard_tile は手牌の末尾（最新ツモ想定）を返す。
    #[test]
    fn test_pick_auto_discard_tile_returns_last() {
        let tiles = vec![
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 2, false),
            Tile::new_number(Suit::Man, 3, false),
        ];
        let picked = pick_auto_discard_tile(&tiles).expect("non-empty");
        assert_eq!(picked, Tile::new_number(Suit::Man, 3, false));
    }

    /// 空手牌なら None。
    #[test]
    fn test_pick_auto_discard_tile_empty() {
        let tiles: Vec<Tile> = Vec::new();
        assert!(pick_auto_discard_tile(&tiles).is_none());
    }
}
