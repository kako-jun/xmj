# 邪雀 Xtreme Mahjong - Web版

WebAssemblyを使用したブラウザ版麻雀ゲームです。

## 必要な環境

- Rust 1.70+
- wasm-pack

## セットアップ

```bash
# wasm-packのインストール
cargo install wasm-pack

# WASMビルド
cd /path/to/xmj
./build-wasm.sh

# または手動でビルド
wasm-pack build --target web --features wasm --out-dir web/pkg
```

## 実行方法

ローカルサーバーを起動してブラウザでアクセス：

```bash
# Pythonの場合
cd web
python3 -m http.server 8000

# Node.jsの場合
npx http-server web -p 8000
```

ブラウザで `http://localhost:8000` にアクセス

## ファイル構成

```
web/
├── index.html          # メインHTML
├── pkg/                # WASM出力ディレクトリ（ビルド時に生成）
│   ├── xmj_core.js
│   ├── xmj_core_bg.wasm
│   └── ...
├── src/                # TypeScriptソース（将来的に使用）
└── public/             # 静的アセット
```

## 機能

- ✅ 基本的な麻雀ゲームプレイ
- ✅ ツモ・打牌
- ✅ シャンテン数表示
- ✅ 手牌表示
- ✅ ゲームログ
- 🚧 CPU対戦
- 🚧 鳴き（チー・ポン・カン）
- 🚧 リーチ
- 🚧 和了判定・点数計算

## 今後の実装予定

- UIフレームワーク統合（React/Svelte）
- 牌の画像表示
- アニメーション効果
- サウンド効果
- オンライン対戦（Nostr + WebRTC）
