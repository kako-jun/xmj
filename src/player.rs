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
    /// 同巡フリテン: ロンを 1 度見逃した直後から、自分の次のツモまで真。
    /// 自分の `draw_tile` で false に戻る (Issue #56)。
    pub skipped_ron_this_turn: bool,
    /// 立直後フリテン: 立直済みの状態でロンを見逃すと永続的にフリテン状態になる。
    /// 局が終わるまで解除されない (`reset_for_next_round` で false に戻る) (Issue #56)。
    pub permanent_furiten: bool,
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
            skipped_ron_this_turn: false,
            permanent_furiten: false,
        }
    }

    pub fn draw_tile(&mut self, tile: Tile) {
        self.hand.add_tile(tile);
        // 自分のツモで同巡フリテン解除 (Issue #56)
        // 永続フリテン (立直後の見逃し由来) は局終了まで解けないのでここでは触らない。
        self.skipped_ron_this_turn = false;
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
        // フリテン関係のフラグも局スコープなので忘れず戻す (Issue #56)
        self.skipped_ron_this_turn = false;
        self.permanent_furiten = false;
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

    /// フリテン判定 (Issue #56)。
    ///
    /// 3 種類を統合して true/false を返す:
    /// 1. **通常フリテン**: 自分の捨て牌に自分の待ち牌のいずれかが含まれていればフリテン
    /// 2. **同巡フリテン**: ロンを 1 度見逃したら自分の次のツモまでフリテン
    ///    (`skipped_ron_this_turn` で管理、`draw_tile` で解除)
    /// 3. **立直後フリテン**: 立直後にロン見逃しが発生したら局終了まで永続フリテン
    ///    (`permanent_furiten` で管理、`reset_for_next_round` で解除)
    ///
    /// フリテン中はロンが不可。ツモは可能なので呼び出し側 (`can_ron` 等) で消費する。
    /// 待ち牌と捨て牌の比較は赤ドラ無視 (`tile_type` のみ比較) で行う。
    pub fn is_furiten(&self) -> bool {
        if self.permanent_furiten {
            return true;
        }
        if self.skipped_ron_this_turn {
            return true;
        }
        // 通常フリテン: 自分の捨て牌に自分の待ち牌のいずれかがあるか
        let waits = self.hand.compute_machi_tiles();
        if waits.is_empty() {
            return false;
        }
        let discards = self.discards_tiles();
        waits
            .iter()
            .any(|w| discards.iter().any(|d| Self::tile_eq_furiten(w, d)))
    }

    /// フリテン用の牌同等比較 (赤ドラを無視して `tile_type` のみで比較)。
    /// 赤 5m と 5m は同じ待ち / 同じ捨て牌として扱う。
    fn tile_eq_furiten(a: &Tile, b: &Tile) -> bool {
        a.tile_type == b.tile_type
    }

    /// ロンを見逃したことを通知する (Issue #56)。
    ///
    /// 「他家の打牌に対して自分がロン可能だったが宣言せず通常進行に戻した」場合に呼ぶ。
    /// - 同巡フリテン: `skipped_ron_this_turn = true`
    /// - 立直済みなら永続フリテン: `permanent_furiten = true`
    ///
    /// 呼び出し側 (TS の skipMeldCall や Game 側) でロン可能状況の判定は済ませてから呼ぶ。
    /// 本関数自体は無条件にフラグを立てるだけ (べき等)。
    pub fn notify_ron_skipped(&mut self) {
        self.skipped_ron_this_turn = true;
        if self.is_riichi {
            self.permanent_furiten = true;
        }
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

    // ==================== can_riichi / declare_riichi 受け入れ条件 (#91) ====================
    //
    // 「リーチ false-positive」バグ調査用テスト群。
    // can_riichi が false を返すべきケース全てで false を返すこと、
    // declare_riichi の戻り値と state 遷移 (is_riichi / score) が一致することを担保する。

    use crate::hand::{Meld, MeldType};
    use crate::tile::Honor;

    /// 13 枚テンパイ手 (タンヤオ平和形 4s/7s 両面待ち) を作る。
    /// `Player::can_riichi` は手牌 13 枚 (打牌直前 = ツモ前) でも、
    /// 14 枚 (ツモ後 = リーチ宣言可否判定の典型タイミング) でも呼ばれうる。
    /// 本テストでは 13 枚状態を基準にする。
    fn make_tenpai_player() -> Player {
        let mut p = Player::new(0, "tester".to_string());
        let tenpai_tiles = vec![
            Tile::new_number(Suit::Man, 2, false),
            Tile::new_number(Suit::Man, 3, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Sou, 2, false),
            Tile::new_number(Suit::Sou, 3, false),
            Tile::new_number(Suit::Sou, 4, false),
            Tile::new_number(Suit::Sou, 5, false),
            Tile::new_number(Suit::Sou, 6, false),
            Tile::new_number(Suit::Man, 8, false),
            Tile::new_number(Suit::Man, 8, false),
        ];
        for t in tenpai_tiles {
            p.hand.add_tile(t);
        }
        p
    }

    #[test]
    fn test_can_riichi_baseline_tenpai_menzen_with_score() {
        let p = make_tenpai_player();
        assert!(p.is_tenpai(), "ベースラインは 13 枚テンパイ");
        assert!(p.can_riichi(), "テンパイ + 門前 + 1000 点以上 + 未リーチ → 立直可能");
    }

    #[test]
    fn test_can_riichi_false_when_not_tenpai() {
        // 明らかに非テンパイ (バラバラの 13 枚)
        let mut p = Player::new(0, "noten".to_string());
        let noten_tiles = vec![
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 5, false),
            Tile::new_number(Suit::Man, 9, false),
            Tile::new_number(Suit::Pin, 1, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 9, false),
            Tile::new_number(Suit::Sou, 1, false),
            Tile::new_number(Suit::Sou, 5, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Nan),
            Tile::new_honor(Honor::Shaa),
            Tile::new_honor(Honor::Pei),
        ];
        for t in noten_tiles {
            p.hand.add_tile(t);
        }
        assert!(!p.is_tenpai(), "国士無双以外の非テンパイ手");
        assert!(!p.can_riichi(), "非テンパイは立直不可");
    }

    #[test]
    fn test_can_riichi_false_when_score_below_1000() {
        let mut p = make_tenpai_player();
        p.score = 500;
        assert!(p.is_tenpai());
        assert!(!p.can_riichi(), "持ち点 < 1000 は立直不可");
        // 境界: ちょうど 1000 は OK
        p.score = 1000;
        assert!(p.can_riichi(), "持ち点 == 1000 は立直可能");
        p.score = 999;
        assert!(!p.can_riichi(), "持ち点 999 は不可");
    }

    #[test]
    fn test_can_riichi_false_when_has_meld() {
        let mut p = make_tenpai_player();
        // 副露を生やす (テンパイ判定上は無視されるが、can_riichi は門前破れで弾く)
        p.hand.add_meld(Meld {
            meld_type: MeldType::Pon,
            tiles: vec![
                Tile::new_honor(Honor::Chun),
                Tile::new_honor(Honor::Chun),
                Tile::new_honor(Honor::Chun),
            ],
            is_open: true,
            from_player: Some(1),
            is_kakan: false,
            claimed_index: Some(0),
        });
        assert!(!p.can_riichi(), "副露ありは立直不可 (門前破れ)");
    }

    #[test]
    fn test_can_riichi_false_when_already_riichi() {
        let mut p = make_tenpai_player();
        assert!(p.declare_riichi(0), "1 回目の立直は成功");
        assert!(p.is_riichi);
        assert!(!p.can_riichi(), "既に立直済みなら立直不可");
        // 2 回目の declare は弾く
        let score_before = p.score;
        assert!(!p.declare_riichi(1), "立直済みからの再宣言は false");
        assert_eq!(p.score, score_before, "失敗時は供託 1000 点を引かない");
    }

    /// 14 枚 (ツモ直後) の non-tenpai 手で is_tenpai/can_riichi が誤って true を返さないか。
    /// Issue #91 の実機症状は「ツモ後にリーチボタンが出る」なので、
    /// 14 枚状態こそ false-positive の主要疑惑タイミング。
    #[test]
    fn test_can_riichi_false_for_14tile_noten() {
        let mut p = Player::new(0, "noten14".to_string());
        // 14 枚バラバラ (筒子・索子・字牌混在、面子要素を最小限に)
        let tiles = vec![
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 5, false),
            Tile::new_number(Suit::Man, 9, false),
            Tile::new_number(Suit::Pin, 1, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 9, false),
            Tile::new_number(Suit::Sou, 1, false),
            Tile::new_number(Suit::Sou, 5, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Nan),
            Tile::new_honor(Honor::Shaa),
            Tile::new_honor(Honor::Pei),
            Tile::new_honor(Honor::Haku),
        ];
        for t in tiles {
            p.hand.add_tile(t);
        }
        assert_eq!(p.hand.get_tiles().len(), 14, "ツモ直後は 14 枚");
        assert!(
            !p.can_riichi(),
            "14 枚バラバラの noten 手で can_riichi=true は false-positive (#91)"
        );
    }

    /// 14 枚で「1 枚捨てればテンパイ」になる手 (ツモ直後に立直可能であるべき正例)。
    /// テンパイ手 + 浮き牌 1 枚 で 14 枚を作る。
    #[test]
    fn test_can_riichi_true_for_14tile_tenpai_drawn() {
        // 13 枚テンパイ手にツモ牌 (浮き牌 9m) を追加 → 14 枚状態
        let mut p = make_tenpai_player();
        p.hand.add_tile(Tile::new_number(Suit::Man, 9, false));
        assert_eq!(p.hand.get_tiles().len(), 14);
        assert!(p.can_riichi(), "14 枚でも 1 枚捨ててテンパイ維持できるなら立直可能");
    }

    #[test]
    fn test_declare_riichi_state_consistent_with_return_value() {
        // 成功ケース: 戻り値 true ⇒ is_riichi=true / score -1000 / ippatsu=true
        let mut p = make_tenpai_player();
        let score_before = p.score;
        assert!(p.declare_riichi(3));
        assert!(p.is_riichi, "戻り値 true なら is_riichi=true");
        assert_eq!(p.score, score_before - 1000, "供託 1000 点引かれている");
        assert!(p.ippatsu, "宣言直後は ippatsu=true");
        assert_eq!(p.riichi_turn, Some(3), "宣言ターンが記録される");

        // 失敗ケース: 戻り値 false ⇒ state は一切変わらない
        let mut p2 = make_tenpai_player();
        p2.score = 500;
        let score_before2 = p2.score;
        assert!(!p2.declare_riichi(0), "score < 1000 は宣言失敗");
        assert!(!p2.is_riichi, "失敗時は is_riichi=false のまま");
        assert_eq!(p2.score, score_before2, "失敗時は供託を引かない");
        assert!(!p2.ippatsu);
        assert_eq!(p2.riichi_turn, None);
    }
}
