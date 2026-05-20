use crate::game::GameMode;
use crate::hand::Hand;
use crate::tile::Tile;

/// 河（捨て牌）の 1 要素。
///
/// 闇麻ルール（`GameMode::Yamima`）対応のため、河を `Vec<Tile>` から
/// `Vec<Discard>` へ拡張した。`is_hidden=true` の牌は他家からは「闇牌」として
/// 種類が見えない（CLI 表示で `??`）。`照射` が成立した時点で
/// `Game::light_up` 経由で `is_hidden=false` に書き換わり公開される。
///
/// `is_hidden` は表示・鳴き不可フラグであり、`tile` 自体は実体として保持する
/// （和了判定・フリテン判定では実体牌として扱う仕様）。
#[derive(Debug, Clone, Copy)]
pub struct Discard {
    pub tile: Tile,
    pub is_hidden: bool,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub id: usize,
    pub name: String,
    pub hand: Hand,
    pub score: i32,
    pub is_dealer: bool,
    pub discards: Vec<Discard>,
    pub is_riichi: bool,
    pub riichi_turn: Option<usize>, // リーチ宣言したターン
    pub ippatsu: bool,               // 一発フラグ
    pub double_riichi: bool,         // ダブル立直
}

impl Player {
    pub fn new(id: usize, name: String) -> Self {
        Self {
            id,
            name,
            hand: Hand::new(),
            score: 25000, // 初期点数
            is_dealer: false,
            discards: Vec::new(),
            is_riichi: false,
            riichi_turn: None,
            ippatsu: false,
            double_riichi: false,
        }
    }

    pub fn draw_tile(&mut self, tile: Tile) {
        self.hand.add_tile(tile);
    }

    pub fn discard_tile(&mut self, tile: Tile) -> bool {
        if self.hand.remove_tile(&tile) {
            self.discards.push(Discard { tile, is_hidden: false });
            true
        } else {
            false
        }
    }

    /// 闇牌打牌（Yamima ルール）。1000 点支払って `is_hidden=true` で河に追加する。
    ///
    /// - 点数が 1000 未満なら false を返して何もしない
    /// - 手牌から `tile` を除去できなければ false（点数も引かない）
    /// - 成功時は 1000 点減 + `Discard { tile, is_hidden: true }` を河に push
    pub fn discard_hidden(&mut self, tile: Tile) -> bool {
        if self.score < 1000 {
            return false;
        }
        if !self.hand.remove_tile(&tile) {
            return false;
        }
        self.score -= 1000;
        self.discards.push(Discard { tile, is_hidden: true });
        true
    }

    /// 河の指定 index の闇牌を公開する（Yamima 照射）。
    ///
    /// - `idx` が範囲外なら None
    /// - 既に公開済み（is_hidden==false）なら None（無効な照射）
    /// - 成功時は `is_hidden=false` に書き換えて該当 tile を返す
    pub fn reveal_discard(&mut self, idx: usize) -> Option<Tile> {
        let d = self.discards.get_mut(idx)?;
        if !d.is_hidden {
            return None;
        }
        d.is_hidden = false;
        Some(d.tile)
    }

    /// 河の Tile のみを抽出（互換ラッパー）。
    ///
    /// 既存コードで「河を Tile のリストとして見たい」読み出し向け。
    /// 公開状態 / 非公開状態を区別せず実体牌だけ取り出す。
    /// 和了判定・フリテン判定で河を走査するときはこれを使う。
    pub fn discards_tiles(&self) -> Vec<Tile> {
        self.discards.iter().map(|d| d.tile).collect()
    }

    pub fn can_win(&self, tile: &Tile) -> bool {
        self.hand.can_win(tile)
    }

    /// モードに応じた和了判定。
    /// - `FiveTile`: `Hand::can_win_five_tile` を呼ぶ
    /// - その他: 既存の `Hand::can_win`（14 枚和了形）を呼ぶ
    pub fn can_win_with_mode(&self, tile: &Tile, mode: GameMode) -> bool {
        match mode {
            GameMode::FiveTile => self.hand.can_win_five_tile(tile),
            _ => self.hand.can_win(tile),
        }
    }

    pub fn get_hand_string(&self) -> String {
        self.hand.to_string()
    }

    pub fn get_discards_string(&self) -> String {
        self.discards
            .iter()
            .map(|d| if d.is_hidden { "??".to_string() } else { d.tile.to_string() })
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn tile_count(&self) -> usize {
        self.hand.tile_count()
    }

    pub fn is_tenpai(&self) -> bool {
        self.hand.is_tenpai()
    }

    pub fn add_score(&mut self, points: i32) {
        self.score += points;
    }

    pub fn subtract_score(&mut self, points: i32) {
        self.score -= points;
        if self.score < 0 {
            self.score = 0;
        }
    }

    /// クランプせず素直に減算する（負スコア許可）。
    ///
    /// 飛び（`score < 0`）検知や、ゼロサム保証が必要な局終了精算で使う。
    /// `subtract_score` は UI 表示の都合で 0 クランプするが、ルール上の点数移動
    /// （`Game::resolve_win` 等）はゼロサム必須なので本 API 経由で処理する。
    pub fn pay_unclamped(&mut self, amount: i32) {
        self.score -= amount;
    }

    /// 次局開始時にリセットすべきプレイヤー状態をまとめて初期化する。
    ///
    /// `Hand::new()` で手牌・副露は別途リセットされるが、
    /// 以下の「局スコープ」フラグはここで明示的に戻す必要がある:
    /// - `is_riichi` / `riichi_turn` / `ippatsu` / `double_riichi`
    /// - 河 (`discards`)
    ///
    /// `score` / `id` / `name` / `is_dealer` は局を跨いで保持するため触らない
    /// （`is_dealer` は `Game::next_round` 側で席ローテーションに合わせて再設定）。
    pub fn reset_for_next_round(&mut self) {
        self.hand = Hand::new();
        self.discards.clear();
        self.is_riichi = false;
        self.riichi_turn = None;
        self.ippatsu = false;
        self.double_riichi = false;
    }

    /// リーチ可能かチェック
    pub fn can_riichi(&self) -> bool {
        // 門前（副露なし）
        if !self.hand.get_melds().is_empty() {
            return false;
        }

        // テンパイ
        if !self.is_tenpai() {
            return false;
        }

        // 1000点以上
        if self.score < 1000 {
            return false;
        }

        // 既にリーチしていない
        !self.is_riichi
    }

    /// リーチを宣言
    pub fn declare_riichi(&mut self, turn: usize) -> bool {
        if !self.can_riichi() {
            return false;
        }

        self.is_riichi = true;
        self.riichi_turn = Some(turn);
        self.ippatsu = true;

        // 供託1000点を支払う
        self.subtract_score(1000);

        true
    }

    /// 一発フラグを消す（鳴きがあった場合など）
    pub fn clear_ippatsu(&mut self) {
        self.ippatsu = false;
    }

    /// 誠京麻雀の役満祝儀を支払う（放銃者）
    ///
    /// `subtract_score` の 0 クランプを意図的に回避してゼロサムを保証する。
    /// 持ち点が祝儀額に満たない場合は **マイナス値を許容する**。
    /// （誠京麻雀の世界観：トビ＝即敗北なので、ゼロ止めではなく素直に負債を計上する）
    pub fn pay_yakuman_tip(&mut self, amount: i32) {
        self.score -= amount;
    }

    /// 誠京麻雀の役満祝儀を受け取る（和了者）
    pub fn receive_yakuman_tip(&mut self, amount: i32) {
        self.add_score(amount);
    }

    /// リーチ後の打牌チェック（ツモ切りのみ）
    pub fn can_discard_after_riichi(&self, tile: &Tile) -> bool {
        if !self.is_riichi {
            return true; // リーチしていない場合は制限なし
        }

        // リーチ後は最後にツモった牌のみ打牌可能
        // 簡易実装: 手牌の最後の牌のみ打牌可能とする
        let tiles = self.hand.get_tiles();
        if let Some(last_tile) = tiles.last() {
            last_tile == tile
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::{Tile, Suit};

    #[test]
    fn test_player_creation() {
        let player = Player::new(0, "Test Player".to_string());
        assert_eq!(player.id, 0);
        assert_eq!(player.name, "Test Player");
        assert_eq!(player.score, 25000);
        assert!(!player.is_dealer);
        assert_eq!(player.tile_count(), 0);
    }

    #[test]
    fn test_draw_and_discard() {
        let mut player = Player::new(0, "Test".to_string());
        let tile = Tile::new_number(Suit::Man, 1, false);

        player.draw_tile(tile);
        assert_eq!(player.tile_count(), 1);

        assert!(player.discard_tile(tile));
        assert_eq!(player.tile_count(), 0);
        assert_eq!(player.discards.len(), 1);
    }

    #[test]
    fn test_yakuman_tip_zero_sum() {
        let mut payer = Player::new(0, "Payer".to_string());
        let mut winner = Player::new(1, "Winner".to_string());

        let payer_before = payer.score;
        let winner_before = winner.score;

        let tip = 8000;
        payer.pay_yakuman_tip(tip);
        winner.receive_yakuman_tip(tip);

        // 双方の差分の合計は 0（payer が 8000 減り、winner が 8000 増える）
        let payer_delta = payer.score - payer_before;
        let winner_delta = winner.score - winner_before;
        assert_eq!(payer_delta + winner_delta, 0);
        assert_eq!(payer_delta, -tip);
        assert_eq!(winner_delta, tip);
    }

    // ========================================
    // Yamima（闇麻）テスト
    // ========================================

    /// `discards_tiles` は Discard ベクタから tile のみ取り出す互換ラッパー。
    /// 公開済 / 闇牌の区別なく実体牌を返す（和了・フリテン判定で使う）。
    #[test]
    fn test_discards_tiles_extracts_tiles_only() {
        let mut player = Player::new(0, "P".to_string());
        let t1 = Tile::new_number(Suit::Man, 1, false);
        let t2 = Tile::new_number(Suit::Pin, 5, false);

        player.draw_tile(t1);
        player.draw_tile(t2);

        assert!(player.discard_tile(t1));
        assert!(player.discard_hidden(t2));

        let extracted = player.discards_tiles();
        assert_eq!(extracted, vec![t1, t2], "公開/闇 を問わず実体牌を順序通り返す");
    }

    /// `discard_hidden` 呼び出しで点数が 1000 減り、河に闇牌として追加される。
    #[test]
    fn test_discard_hidden_costs_1000() {
        let mut player = Player::new(0, "P".to_string());
        let tile = Tile::new_number(Suit::Sou, 3, false);
        player.draw_tile(tile);

        let score_before = player.score;
        assert!(player.discard_hidden(tile));
        assert_eq!(player.score, score_before - 1000, "1000 点支払う");
        assert_eq!(player.discards.len(), 1);
        assert!(player.discards[0].is_hidden, "is_hidden=true で河に追加される");
        assert_eq!(player.discards[0].tile, tile, "実体牌は保存される");
    }

    /// 点数 999 以下では `discard_hidden` は失敗する（点数も手牌も変動しない）。
    #[test]
    fn test_discard_hidden_fails_when_score_below_1000() {
        let mut player = Player::new(0, "P".to_string());
        let tile = Tile::new_number(Suit::Man, 1, false);
        player.draw_tile(tile);
        player.score = 999;

        assert!(!player.discard_hidden(tile));
        assert_eq!(player.score, 999, "失敗時は点数を引かない");
        assert_eq!(player.tile_count(), 1, "失敗時は手牌も減らない");
        assert!(player.discards.is_empty());
    }

    /// `reveal_discard` は闇牌を公開して tile を返す（照射成立）。
    #[test]
    fn test_reveal_discard_returns_tile() {
        let mut player = Player::new(0, "P".to_string());
        let tile = Tile::new_number(Suit::Pin, 7, false);
        player.draw_tile(tile);
        assert!(player.discard_hidden(tile));

        let revealed = player.reveal_discard(0);
        assert_eq!(revealed, Some(tile), "実体牌が返る");
        assert!(!player.discards[0].is_hidden, "is_hidden=false に書き換わる");
    }

    /// 既に公開済みの河に対する `reveal_discard` は None（無効な照射）。
    #[test]
    fn test_reveal_already_revealed_returns_none() {
        let mut player = Player::new(0, "P".to_string());
        let tile = Tile::new_number(Suit::Sou, 1, false);
        player.draw_tile(tile);
        assert!(player.discard_tile(tile)); // 通常打牌（is_hidden=false）

        assert_eq!(player.reveal_discard(0), None, "公開済は照射対象外");
    }

    /// 河の表示: 闇牌は `??`、公開済は通常表示。
    #[test]
    fn test_get_discards_string_masks_hidden() {
        let mut player = Player::new(0, "P".to_string());
        let t1 = Tile::new_number(Suit::Man, 1, false);
        let t2 = Tile::new_number(Suit::Pin, 5, false);
        player.draw_tile(t1);
        player.draw_tile(t2);

        assert!(player.discard_tile(t1));
        assert!(player.discard_hidden(t2));

        let s = player.get_discards_string();
        // t1 は表示、t2 は ??
        assert!(s.contains(&t1.to_string()), "公開済は牌名が見える");
        assert!(s.contains("??"), "闇牌は ?? で表示される");
        assert!(!s.contains(&t2.to_string()), "闇牌の中身は表示されない");
    }

    /// 役満祝儀はマイナス点になっても 0 クランプしないことを保証する（ゼロサム維持）
    #[test]
    fn test_yakuman_tip_can_go_negative() {
        let mut payer = Player::new(0, "Payer".to_string());
        let mut receiver = Player::new(1, "Receiver".to_string());

        // 持ち点を意図的に低くする
        payer.score = 3000;
        receiver.score = 25000;

        let sum_before = payer.score + receiver.score;

        payer.pay_yakuman_tip(8000);
        receiver.receive_yakuman_tip(8000);

        // payer は -5000（クランプされない）
        assert_eq!(payer.score, -5000, "0 クランプされずにマイナスに突き抜ける");
        assert_eq!(receiver.score, 33000, "receiver は 8000 増える");

        // ゼロサム不変
        assert_eq!(payer.score + receiver.score, sum_before, "ゼロサムが維持される");
    }
}
