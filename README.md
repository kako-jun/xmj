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
cargo run
```

ゲームが起動したら、手牌から打牌する牌を入力してください:

```
打牌する牌を入力してください (例: 1m, 5p, to): 1m
```

**牌の入力形式**:
- 数牌: `1m`～`9m`（萬子）、`1p`～`9p`（筒子）、`1s`～`9s`（索子）
- 赤ドラ: `5mr`, `5pr`, `5sr`
- 字牌: `to`（東）, `na`（南）, `sa`（西）, `pe`（北）, `hk`（白）, `ht`（発）, `cn`（中）

### Web版（開発中）

```bash
cd web
npm install
npm run dev
```

ブラウザで `http://localhost:5173` にアクセスしてください。

## ゲームモード

### 実装済み

- ✅ **通常麻雀**: 標準的なリーチ麻雀（開発・デバッグ用）

### 実装予定

- 🚧 **誠京麻雀**: 場代、二度ヅモ、役満祝儀（『銀と金』）
- 🚧 **鷲巣麻雀**: ガラス牌、血液ポイント（『アカギ』）
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
│   └── ai.rs           # AI思考エンジン（予定）
├── web/                # Webクライアント（予定）
├── .claude/            # 開発ドキュメント
│   ├── vision.md       # プロジェクトビジョン
│   ├── design.md       # 技術設計
│   └── todo.md         # 実装TODOリスト
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
