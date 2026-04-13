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
}
```

**機能**:
- 山牌の生成とシャッフル
- 配牌（親14枚、子13枚）
- ツモ・打牌の管理
- ターン制御
- ロン判定
- ゲーム終了判定

### 5. Scoring Engine (`src/scoring.rs`)

**責務**: 役判定と点数計算

```rust
pub enum Yaku {
    Riichi, Tanyao, Pinfu, Yakuhai(Honor),
    // ... 他の役
}

pub struct ScoringResult {
    pub han: u32,
    pub fu: u32,
    pub yaku: Vec<Yaku>,
    pub base_points: u32,
    pub total_points: u32,
}
```

**機能**:
- 役判定（リーチ、タンヤオ、ピンフ、役牌、etc.）
- 符計算
- 飜数計算
- 点数計算（満貫、跳満、倍満、役満対応）
- 親子・ツモロンの得点差分

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
}
```

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
