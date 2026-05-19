# 邪雀 Xtreme Mahjong (xmj)

**「流れ」「オカルト」「極限の駆け引き」をテーマにした異端の麻雀ゲーム**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](#)

## 概要

「邪雀 Xtreme Mahjong (xmj)」は、福本伸行作品（『アカギ』『銀と金』『天』など）に登場するような緊張感とカオスに満ちた対局体験を提供する、新しい形の麻雀ゲームです。

### 特徴

- 🎲 **多彩な特殊ルール**: 鷲巣麻雀、誠京麻雀、闇麻、リアルタイム麻雀など
- 🌐 **P2P分散型**: NostrとWebRTCによるサーバーレスオンライン対戦
- 🖥️ **クロスプラットフォーム**: CUI（ターミナル）とWebブラウザの両対応
- 🔓 **即時プレイ**: アカウント登録不要、匿名でのオンライン対戦
- 🤖 **公平なAI**: プレイヤーと同じ情報のみで思考するCPU対戦
- 🦀 **Rust製**: 高速で安全なコアロジック、WebAssemblyでWeb展開

## インストール

### 必要環境

- Rust 1.70+ (CUI版)
- Node.js 18+ (Web版、開発時のみ)

### CUI版のビルド

```bash
# リポジトリのクローン
git clone https://github.com/kako-jun/xmj.git
cd xmj

# ビルドと実行
cargo build --release
cargo run --release
```

## 使い方

### CUIでのプレイ

```bash
# 標準ルール
cargo run

# 誠京麻雀ルール（場代・二度ヅモ・役満祝儀）
cargo run -- --mode seikyo

# 鷲巣麻雀ルール（3/4 透明牌、他家の glass 牌が見える）
cargo run -- --mode washizu
```

ゲームが起動したら、手牌から打牌する牌を入力してください:

```
打牌する牌を入力してください (例: 1m, 5p, to): 1m
```

**牌の入力形式**:
- 数牌: `1m`～`9m`（萬子）、`1p`～`9p`（筒子）、`1s`～`9s`（索子）
- 赤ドラ: `5mr`, `5pr`, `5sr`
- 字牌: `to`（東）, `na`（南）, `sa`（西）, `pe`（北）, `hk`（白）, `ht`（発）, `cn`（中）

### Web版

#### ローカルCPU対戦

```bash
# WASMビルド
./build-wasm.sh

# ローカルサーバー起動
cd web
python3 -m http.server 8000
```

ブラウザで各モードにアクセス:
- `http://localhost:8000` - 4人CPU対戦（観戦モード）
- `http://localhost:8000/hybrid.html` - **ハイブリッドモード（1人間 + 3CPU）** ⭐おすすめ
- `http://localhost:8000/matchmaking.html` - P2Pマッチング画面
- `http://localhost:8000/debug.html` - 開発者デバッグツール

**ハイブリッドモードの特徴**:
- 👤 あなた1人 vs 🤖 CPU 3人で対戦
- 🎯 東家・南家・西家・北家から席を選択可能
- ⚡ P2P通信なしで即座にプレイ開始
- 🐛 1台のPCでデバッグ・テストが可能

#### オンラインP2P対戦（開発中）

```bash
# マッチング画面
# ブラウザで http://localhost:8000/matchmaking.html にアクセス
```

**P2P対戦の特徴**:
- 🔑 Nostr鍵ペアによる匿名認証
- 🌐 Nostrリレーでのマッチング
- 🔗 WebRTCによる低遅延P2P通信
- 🎮 4人対戦対応（メッシュトポロジー）
- 🏠 サーバーレス（中央サーバー不要）

**注意**: 現在P2P機能は基礎実装のみで、実際のゲームプレイはまだ実装されていません。

## ゲームモード

### 実装済み

- ✅ **通常麻雀**: 標準的なリーチ麻雀（開発・デバッグ用）
  - ✅ 完全な和了判定（4面子1雀頭、七対子、国士無双）
  - ✅ 全役判定（一飜～役満）
  - ✅ シャンテン数計算
  - ✅ 鳴き（チー・ポン・カン）
  - ✅ リーチシステム
  - ✅ AIエンジン（3レベル）

- ✅ **誠京麻雀** (`--mode seikyo`): 場代1000点、二度ヅモ、役満祝儀8000点（『天』『アカギ』）
  - ✅ 場代: API 提供（`collect_seat_fee` / `winner_takes_pot`）。`main.rs` ではゲーム開始時に 1 回供託する **simplified version**。局ごとの再徴収は follow-up
  - ✅ 親二度ヅモ: API 提供（`dealer_double_draw`）+ CUI で 1 枚目を即捨てる UX。連荘フラグ（`dealer_won_last`）の自動更新配線は follow-up
  - ✅ 役満祝儀: API 提供（`pay_yakuman_tip` / `receive_yakuman_tip`、ゼロサム保証）。実際の役満和了時の自動授受配線は follow-up

- ✅ **鷲巣麻雀** (`--mode washizu`): 3/4 透明牌、他家の glass 牌が見える（『アカギ』）
  - ✅ 透明牌: `Tile.is_glass` フラグ + `initialize_wall` で全 136 牌のうち 3/4 (102 枚) を glass 化
  - ✅ 可視性 API: `Game::get_visible_tiles_of_opponent(observer, target)` で他家手牌のうち glass 牌のみ取得
  - ✅ CLI 表示: Washizu モード時のみ `[CPUx の透明牌: ...]` を各他家に追加表示。Standard/Seikyo の表示には影響しない
  - 🚧 血液ポイント・牌の使用回数制限などの追加ルールは follow-up

#### Limitations（誠京麻雀の現状）

PR #21 時点の実装は API レベル完備 + CLI からの最低限の動線確認まで。以下は未配線で follow-up Issue で対応予定:

- **局ループ未実装**: xmj には現状「局終了→次局」のループが無いため、場代は各局開始時ではなく **ゲーム開始時に 1 回だけ** 徴収される。Issue #16 仕様との差分として明示
- **連荘フラグの自動更新**: `dealer_won_last` は外部から win-resolve 時に手動で更新するフラグ。本番和了フローからの自動更新は未配線
- **役満祝儀の自動授受**: 役満和了判定 → 放銃者特定 → 自動授受、の本番配線は未実装。`pay_yakuman_tip` / `receive_yakuman_tip` は API のみ提供（ツモ・ロン両対応）
- **場代の親回収帰属**: 標準解釈を採用（誰が和了しても pot は和了者が回収）。親回収バリアントは将来オプション化検討
- **供託の流局持ち越し**: `winner_takes_pot` を呼ばなければ pot は自然に持ち越されるため、流局処理ロジックさえ書けば対応可能

### 実装予定

- 🚧 **闇麻**: 闇牌、照射
- 🚧 **リアルタイム麻雀**: 同時打牌、早い者勝ちの鳴き
- 🚧 **東西戦**: クリア麻雀（指定役5つの達成競争）

## プロジェクト構成

```
xmj/
├── src/
│   ├── lib.rs          # ライブラリエントリーポイント
│   ├── main.rs         # CUIクライアント
│   ├── tile.rs         # 牌システム
│   ├── hand.rs         # 手牌管理
│   ├── player.rs       # プレイヤー管理
│   ├── game.rs         # ゲーム進行
│   ├── scoring.rs      # 役判定・点数計算
│   ├── ai.rs           # AI思考エンジン
│   ├── nostr.rs        # Nostr P2P通信（ネイティブ）
│   ├── wasm.rs         # WASMバインディング
│   ├── wasm_nostr.rs   # Nostr P2P通信（WASM）
│   └── wasm_webrtc.rs  # WebRTC P2P通信（WASM）
├── web/                # Webクライアント
│   ├── index.html      # CPU対戦（観戦モード）
│   ├── hybrid.html     # ハイブリッドモード（1人間+3CPU）
│   ├── matchmaking.html # P2Pマッチング画面
│   ├── debug.html      # 開発者デバッグツール
│   ├── pkg/            # WASMビルド出力
│   └── README.md       # Web版説明
├── .claude/            # 開発ドキュメント
│   ├── vision.md       # プロジェクトビジョン
│   ├── design.md       # 技術設計
│   └── todo.md         # 実装TODOリスト
├── build-wasm.sh       # WASMビルドスクリプト
├── CLAUDE.md           # 総合プロジェクトドキュメント
├── Cargo.toml          # Rust設定
└── README.md           # このファイル
```

## 技術スタック

### コアエンジン
- **言語**: Rust
- **ビルド**: Cargo

### CUIクライアント
- **UI**: ターミナル標準入出力

### Webクライアント（予定）
- **言語**: TypeScript
- **フレームワーク**: React / Svelte / Vue（選定中）
- **ビルドツール**: Vite
- **WASM**: wasm-bindgen

### P2P通信（予定）
- **プロトコル**: Nostr + WebRTC
- **マッチング**: Nostr Relay
- **ゲーム通信**: WebRTC DataChannel

## 開発状況

### ✅ フェーズ1: コアロジックとCUIプロトタイプ（完了）

- Rustで麻雀コアライブラリ実装
- CUIクライアント作成
- CPU対戦機能（シンプル）
- 基本的な役判定・点数計算

### 🚧 現在の作業

- 役判定の完全実装（全役対応）
- 和了判定の改善（正確な面子構成判定）
- AI思考エンジンの作成

### 📋 今後の予定

- フェーズ2: Nostr P2P通信基盤
- フェーズ3: Web版クライアント開発
- フェーズ4: オンライン対戦機能
- フェーズ5: 特殊ルール実装

詳細は [.claude/todo.md](.claude/todo.md) を参照してください。

## コントリビューション

現在は個人開発中ですが、将来的にコントリビューションを歓迎する予定です。

## ライセンス

MIT License - 詳細は [LICENSE](LICENSE) を参照してください。

## 作者

- **kako-jun** - [GitHub](https://github.com/kako-jun)

## 謝辞

- 福本伸行先生の作品群にインスパイアされました
- Nostrプロトコルと分散型技術コミュニティに感謝

---

**邪雀で、常識を超えた麻雀体験を。**
