# 邪雀 Xtreme Mahjong - 技術設計ドキュメント

## システムアーキテクチャ

### レイヤー構成

```
┌─────────────────────────────────────────────────┐
│         Presentation Layer                      │
│  ┌──────────────┐      ┌──────────────────┐   │
│  │ CUI Client   │      │   Web Client     │   │
│  │  (Rust)      │      │ (TS/React/Svelte)│   │
│  └──────────────┘      └──────────────────┘   │
└─────────────────────────────────────────────────┘
                    │              │
                    └──────┬───────┘
                           │
┌─────────────────────────────────────────────────┐
│         Core Logic Layer (Rust)                 │
│  ┌──────────────────────────────────────────┐  │
│  │  Game Engine (Native / WASM)             │  │
│  │  - Tile System                           │  │
│  │  - Hand Management                       │  │
│  │  - Scoring Engine                        │  │
│  │  - Game Flow Control                     │  │
│  │  - AI Engine                             │  │
│  └──────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────┐
│      Communication Layer                        │
│  ┌──────────────┐      ┌──────────────────┐   │
│  │    Nostr     │      │    WebRTC        │   │
│  │ (Matching &  │      │  (P2P Game       │   │
│  │  Signaling)  │      │   Messaging)     │   │
│  └──────────────┘      └──────────────────┘   │
└─────────────────────────────────────────────────┘
```

## コアモジュール設計

### 1. Tile System (`src/tile.rs`)

**責務**: 牌の表現と操作

```rust
pub enum Suit { Man, Pin, Sou }
pub enum Honor { Ton, Nan, Shaa, Pei, Haku, Hatsu, Chun }
pub enum TileType {
    Number { suit: Suit, value: u8 },
    Honor(Honor),
}
pub struct Tile {
    pub tile_type: TileType,
    pub is_red: bool,  // 赤ドラ
}
```

**機能**:
- 文字列⇔牌オブジェクト変換 (`1m`, `5pr`, `to`)
- 牌の比較・ハッシュ化
- 牌の表示

### 2. Hand Management (`src/hand.rs`)

**責務**: 手牌と副露の管理

```rust
pub struct Hand {
    tiles: Vec<Tile>,
    melds: Vec<Meld>,  // チー、ポン、カン
}

pub enum MeldType { Chi, Pon, Kan }
pub struct Meld {
    pub meld_type: MeldType,
    pub tiles: Vec<Tile>,
    pub is_open: bool,
}
```

**機能**:
- 手牌の追加・削除
- 自動ソート
- 副露管理
- テンパイ判定
- 和了判定

### 3. Player (`src/player.rs`)

**責務**: プレイヤー状態の管理

```rust
pub struct Player {
    pub id: usize,
    pub name: String,
    pub hand: Hand,
    pub score: i32,
    pub is_dealer: bool,
    pub discards: Vec<Tile>,  // 河（捨て牌）
}
```

**機能**:
- ツモ・打牌
- 点数管理
- 河の管理
- 和了判定

### 4. Game Engine (`src/game.rs`)

**責務**: ゲーム全体の進行管理

```rust
pub struct Game {
    pub players: Vec<Player>,
    pub wall: Vec<Tile>,         // 山牌
    pub dora_indicators: Vec<Tile>,
    pub current_player: usize,
    pub round: u32,
    pub dealer: usize,
    pub last_discard: Option<Tile>,
    pub length: Length,          // 東風戦 / 半荘戦
    pub honba: u32,              // 本場
    pub riichi_sticks: u32,      // 供託リーチ棒の本数
    pub last_outcome: Option<RoundOutcome>,
    pub game_over: bool,
    // (mode / pot / dealer_won_last / team_progress / player_timers 等は省略)
}
```

**機能**:
- 山牌の生成とシャッフル
- 配牌（親14枚、子13枚）
- ツモ・打牌の管理
- ターン制御
- ロン判定
- 局ループ (`resolve_win` / `resolve_draw` / `next_round`)
- ゲーム終了判定 (`is_game_over`: 飛び / 規定局終了 / EastWest クリア)

### 5. Scoring Engine (`src/scoring.rs` + `src/agari.rs` + `src/yaku_struct.rs`)

**責務**: 役判定と点数計算

```rust
pub enum Yaku {
    Riichi, Tanyao, Pinfu, Yakuhai(Honor),
    // ... 標準役 + 役満 + ローカル役 (OpenRiichi/Renhou/Daisharin/Suurenkou/Hyakumangoku/Sanrenkou)
}

pub struct ScoringResult {
    pub han: u32,
    pub fu: u32,
    pub yaku: Vec<Yaku>,
    pub base_points: u32,
    pub total_points: u32,
    pub dora: u32, pub uradora: u32, pub akadora: u32, pub kandora: u32,
    pub yakuman_count: u32, // 0=非役満、1=単役満、2=ダブル…
}
```

**モジュール構成** (#108 監査で分解ベースに刷新):
- `scoring.rs` — `ScoringEngine::calculate_score_with_context(hand, winning_tile, ctx)` が live path。役満・牌集合ベース役 (タンヤオ/混一/清一/トイトイ/役牌/混老頭) + ドラ + 点数計算を担う。`ScoringContext` で立直系/状況役/場風自風/ドラ/各種ローカルトグルを受け取る。
- `agari.rs` — 和了形分解エンジン。`enumerate_concealed_decomps(tiles, winning, melds_needed)` が (雀頭 + N面子 + 待ち形) を全列挙。副露あり手対応に一般化済み。赤ドラは分解前に正規化。平和形 / 四暗刻 (`is_suuankou_n`、暗槓対応) / 九蓮の判定ヘルパーを提供。
- `yaku_struct.rs` — 面子分解ベースの構造役と符計算。一盃口/二盃口/三色同順/一気通貫/チャンタ/純チャン/三色同刻/小三元/三暗刻/平和/三連刻 + 符 (面子符/待ち符/雀頭役牌符/門前ロン/平和20-30/喰い平和形ロン30) を算出し、通常形と七対子で高得点の解釈を採用。

> ⚠️ 旧実装では構造役8種がスタブ (常に false/None)・符計算がスタブ (基本符20+ツモ2のみ) だった。詳細は `docs/audit-scoring.md`。

**機能**:
- 役判定（標準役・役満・ローカル役満・状況役）
- 符計算（面子符・待ち符・雀頭符・門前ロン・平和/七対子特殊符）
- 点数計算（満貫頭打ち・跳満・倍満・三倍満・数え役満・倍役満対応）
- 親子・ツモロンの得点差分、本場・供託

**特殊 / ローカルルール** (session547 で実装、`Game` のトグル経由。詳細は `DESIGN.md` §10):
- 食い替え禁止 (#59) / 喰いタン toggle (#129) / ローカル役満 (#58) / オープンリーチ (#60) /
  本場縛り (#61) / 包=責任払い (#57) / 割れ目 (#118) / 特殊流局 (#55) / 差し馬 (#117)。
- 役満確定打牌の責任払い (`Game::check_pao_after_call` → `apply_pao_payment`)、割れ目の支払い 2 倍、
  流し満貫 (`nagashi_mangan_players` → `resolve_draw`)、途中流局 (`RoundOutcome::AbortiveDraw`) を含む。

## P2P通信設計

### Nostr + WebRTC ハイブリッドモデル

#### フェーズ1: マッチング（Nostr）

1. プレイヤーが「対戦募集」イベントを送信
   ```json
   {
     "kind": 30001,
     "tags": [
       ["d", "game-xmj"],
       ["type", "seeking-match"],
       ["game_mode", "normal"],
       ["player_count", "4"]
     ],
     "content": "..."
   }
   ```

2. 他プレイヤーが「参加応答」イベントを返信
   ```json
   {
     "kind": 30001,
     "tags": [
       ["d", "game-xmj"],
       ["type", "join-request"],
       ["e", "<募集イベントID>"]
     ],
     "content": "..."
   }
   ```

3. シグナリング情報（SDP, ICE Candidate）をNostr経由で交換

#### フェーズ2: ゲームプレイ（WebRTC）

1. WebRTC DataChannelでP2P接続確立
2. ゲームイベント（打牌、ツモ、ロンなど）を直接通信
3. 軽量バイナリフォーマットで低遅延通信

#### フェーズ3: ログ・戦績（Nostr）

1. 対戦結果をNostrイベントとして記録
2. クライアント側で集計してランキング表示

### Nostrイベント設計

```
kind: 30001 (ゲームイベント)
tags:
  - ["d", "game-xmj"]              # 識別子
  - ["gid", "<ゲームID>"]          # ゲームセッションID
  - ["type", "discard|draw|ron"]   # イベントタイプ
  - ["tile", "1m"]                 # 牌情報
  - ["player", "<公開鍵>"]         # プレイヤー識別
```

## AI設計

### 公平性の原則

AIは以下の情報のみを参照:
- 自身の手牌
- 全プレイヤーの河（捨て牌）
- ドラ表示牌
- 鳴きの情報
- 点数状況

**禁止事項**:
- 山牌の盗み見
- 他家の手牌の盗み見

### AI戦略レベル

1. **レベル1 (簡易)**:
   - ランダムまたは最初の牌を打つ

2. **レベル2 (基本)**:
   - 孤立牌優先打
   - 危険牌回避

3. **レベル3 (中級)**:
   - 向聴数計算
   - 手役狙い

4. **レベル4 (上級)**:
   - 期待値計算
   - 押し引き判断
   - 読み（河から推測）

## WebAssembly (WASM) 設計

### ビルド構成

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
```

### JS/TSインターフェース

```rust
#[wasm_bindgen]
pub struct WasmGame {
    game: Game,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new(player_names: Vec<String>) -> Self { ... }

    pub fn draw_tile(&mut self) -> bool { ... }
    pub fn discard_tile(&mut self, tile_str: &str) -> bool { ... }
    pub fn get_game_state(&self) -> String { ... }

    // --- Round loop bridge (Issue #27) ---
    pub fn resolve_draw(&mut self, tenpai_player_indices: Vec<usize>);
    pub fn resolve_win_tsumo(&mut self, winner_idx: usize) -> String; // 役/han/fu/totalPoints の JSON
    pub fn resolve_win_ron(&mut self, winner_idx: usize, from_idx: usize) -> String;
    pub fn next_round(&mut self) -> bool;          // true=続行 / false=終局
    pub fn get_round(&self) -> u32;
    pub fn get_honba(&self) -> u32;
    pub fn get_dealer(&self) -> usize;
    pub fn get_riichi_sticks(&self) -> u32;
    pub fn get_last_outcome_json(&self) -> String; // "" or {kind:"win"|"draw", ...}
}
```

`resolve_win_tsumo` は内部で `ScoringEngine::calculate_score` を呼ぶ。winning_tile は
手牌 14 枚から「抜くと残りが和了形になる 1 枚」を探索することで決定する（`Hand` は
add_tile 時に自動ソートされるため末尾位置から復元できないため）。
和了形でない手牌に対しては空文字を返し、`last_outcome` は更新しない（呼び出し側の
安全網）。

TS 側のラッパは `web/src/game/wasm.ts` の `WasmGameBridge` および
`web/src/game/types.ts` の `RoundOutcome` / `parseRoundOutcome`。UI は
`web/src/game/roundResultScene.ts` の中間結果シーンを描画し、「次局へ」
ボタンで `nextRound()` を呼ぶ。役満ご祝儀 (#28) / 東西戦 team_yaku (#29) /
和了 UI ボタンの本実装は別 Issue。

## セキュリティ設計

### Nostr署名による認証

- 各プレイヤーはNostr秘密鍵で操作に署名
- 他プレイヤーは公開鍵で署名を検証
- なりすまし防止

### 不正防止

1. **クライアント側検証**:
   各プレイヤーのクライアントが全員の操作を検証

2. **合意形成**:
   4人全員が同じゲーム状態を持つことを確認

3. **切断対応**:
   タイムアウト時の自動処理

## データ永続化

### ローカルストレージ

- Nostr鍵ペア
- ゲーム設定
- 戦績履歴

### Nostrリレー（分散ストレージ）

- グローバル戦績
- ランキング
- リプレイデータ

## UI/UX設計

### CUI版

```
Round: 1 | Wall: 70 tiles
Dora indicators: 5p

>親 あなた (25000点): 1m 2m 3m 4p 5p 6p 7s 8s 9s to to na na
  河: 9m 1p

  CPU1 (25000点): [13 tiles]
  河: 1s 9p

  CPU2 (25000点): [13 tiles]
  河: to

  CPU3 (25000点): [13 tiles]
  河: 1m 9s

Last discard: 9s
```

### Web版

```
┌─────────────────────────────────────────────┐
│            北家 (CPU3)                      │
│         点数: 25000                         │
└─────────────────────────────────────────────┘

┌──────┐                          ┌──────┐
│西家   │                          │東家   │
│(CPU2)│                          │(あなた)│
│25000 │       [ドラ: 5p]         │25000 │
└──────┘                          └──────┘

┌─────────────────────────────────────────────┐
│            南家 (CPU1)                      │
│         点数: 25000                         │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ [手牌]                                      │
│ 1m 2m 3m 4p 5p 6p 7s 8s 9s to to na na     │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ [ログ・チャット]                            │
│ [システム] あなたがツモ: 3m                │
│ [CPU1] よろしくお願いします                │
│ [システム] CPU1が打牌: 1s                  │
│ [あなた] よろしく                          │
└─────────────────────────────────────────────┘
```

## テスト戦略

### ユニットテスト
- 各モジュールの単体機能テスト
- カバレッジ目標: 80%以上

### 統合テスト
- ゲームフロー全体のテスト
- P2P通信のシミュレーション

### E2Eテスト
- CUI版の実際のプレイテスト
- Web版のブラウザテスト
