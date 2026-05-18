# xmj Web (PixiJS v8 + Wasm)

邪雀 Xtreme Mahjong の Web フロントエンド。Rust 製のゲームロジック (`../src/`) を
WebAssembly 経由で呼び出し、PixiJS v8 で描画する。

## 開発

```bash
npm install
npm run dev       # http://localhost:3000
```

`npm run dev` / `build` / `lint` / `typecheck` / `test` の前段で自動的に
`../build-wasm.sh` が走り、`web/pkg/` を再生成する (name-name 流の sync-wasm)。

## ディレクトリ構成

```
web/
├── index.html        # PixiJS マウントポイント + ローディングバー
├── src/
│   ├── main.ts       # Pixi.Application 起動
│   └── game/
│       ├── App.ts        # シーン基盤 (Issue #5 で SceneManager 化予定)
│       ├── constants.ts  # ステージ・牌サイズ・色
│       ├── types.ts      # Tile / GameState / CUI 文字列変換
│       ├── state.ts      # createInitialGameState / initWithState
│       ├── wasm.ts       # initWasm + WasmGameBridge
│       └── tile.ts       # createTileGraphics / 裏向き / 全牌列挙
├── pkg/              # wasm-pack 出力 (gitignore)
└── legacy/           # 旧 HTML プロトタイプ (参照用、ビルド対象外)
```

詳細は ルート `CLAUDE.md` の「6. Web 版」セクション、移植進捗は `../docs/migration-pixijs.md` を参照。

## テスト

```bash
npm run test
```

vitest + jsdom。PixiJS の WebGL レンダラは jsdom では起動しないため、
ロジック層 (state / wasm ラッパ / tile factory) を中心にテストする。
