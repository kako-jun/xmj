# PixiJS v8 移植進捗

旧 HTML/JS プロトタイプ (`web/legacy/`) から、PixiJS v8 + Vite + Wasm 構成への置き換え進捗を集約する。

## 背景

旧 `web/` は素の HTML + 直接 DOM 操作で組まれており、テキストノードで牌を描画していた。Issue #2 から PixiJS ベースの新フロントエンドに作り替え、CUI と Web の二系統 + Wasm コア共有という最終形に向けて土台を整え直している。

旧コードは `web/legacy/` に退避済み (ビルド対象外、参照用)。新規開発はすべて `web/src/` 配下で行う。

## 完了済み Issue

### Issue #2 / PR #10 — PixiJS v8 + Vite + Wasm 環境構築

- PixiJS 8 / Vite 6 / TypeScript 5.7 / vitest 4 / ESLint 9 / Prettier 3
- `vite-plugin-wasm` + `vite-plugin-top-level-await` で wasm-pack (--target web) 出力を import
- `npm run` の `pre*` フックで `../build-wasm.sh` を自動実行 (name-name 流 sync-wasm)
- `web/pkg/` は gitignore (build-wasm.sh で再生成される副産物)
- スモークテスト: `App.test.ts` で stage に背景レイヤが追加されることを確認

### Issue #3 / PR #11 — GameState 型 + initWithState + WasmGameBridge

- `web/src/game/types.ts`: `Tile` / `Suit` / `PlayerState` / `GameState` / `GamePhase`
- 風牌 = 東1/南2/西3/北4、三元 = 白1/發2/中3 で Rust 側 `tile.rs` と整合
- `tileToCuiCode` / `tileFromCuiCode`: CUI 表記 (`1m` / `5pr` / `to` / `hk` etc.) と相互変換
- `web/src/game/state.ts`: `createInitialGameState()` / `initWithState(partial)`
- `web/src/game/wasm.ts`: `WasmGameBridge` クラス。Rust 側 `WasmGame` の全 API をラップ
- テスト 17 件 (types ラウンドトリップ / state マージ / wasm モック注入)

### Issue #4 / PR #12 — 牌の PixiJS Graphics 実装

- `web/src/game/tile.ts`: `createTileGraphics(tile)` / `createTileBackGraphics()` / `enumerateAllTiles()`
- 角丸長方形 (アイボリー) + 縁取り + 上段グリフ + 下段スート漢字
- 索子 = 緑 (`#117733`) / 筒子 = 青 (`#1e4e8c`) / 萬子・字牌 = 黒 / 赤ドラ = 赤 (`#c1121f`)
- `Container.label` に CUI コードを埋め、テスト・デバッグから参照可
- 動作確認: `App.showAllTilesDemo()` で 34 種 + 赤ドラ 3 枚を 9 列グリッドで表示
- テスト 12 件 (列挙数 / 生成 / label / Text 内容 / 文字色)

### Issue #5 — 初期配牌卓の PixiJS 描画

- `web/src/game/table.ts`: 4 方向の手牌、中央情報盤、河スロット、スコア帯をまとめた卓シーンを追加
- `web/src/game/bridgeState.ts`: Rust 側 `getGameState()` の整形文字列を `GameState` に戻す最小パース層を追加
- `web/src/main.ts`: `WasmGameBridge.createHybrid('あなた', 0)` から初期局面を生成し、旧 `showAllTilesDemo()` を本番起動から除去
- 相手手牌は文字列から枚数を復元しつつ、描画は裏向きで固定
- 山牌残数、ドラ表示牌、東1局表示、空の河スロット、持ち点/手番表示まで初期卓に反映
- テスト追加: 卓構造スモーク、整形文字列からの初期局面変換

### Issue #5 フォローアップ — レビュー指摘対応

- `createGameStateFromBridge()` は `getCurrentHandString()` を「現在手番が人間席のときだけ」適用するよう修正
- これにより、人間席が非手番の局面で CPU の current hand が誤って人間席へ流れ込むバグを防止
- `App.showInitialTable()` は再描画時に旧卓 `Container` を `destroy({ children: true })` まで行い、リークしにくい形へ変更
- `DESIGN.md` を現行 PixiJS 初期卓表示に合わせて更新し、戦闘卓は暗いフェルト卓、gradient / glass は DOM overlay 用、牌は vector + text glyph という方針を明文化

### Issue #6 — 入力（牌選択・打牌・ツモのタップ操作）

- `web/src/game/App.ts`: `WasmGameBridge` を保持する構造へ寄せ、`selectedHandIndex` / CPU ターン進行 / 人間復帰時の自動ツモを集約
- 人間手番かつ下側の手牌だけ `pointertap` を有効化。1 回タップで選択、同じ牌の再タップで `discardTile()` を呼ぶ
- 打牌後は `createGameStateFromBridge()` で UI state を再構築し、CPU が手番の間は `executeCpuTurn()` を順に実行
- CPU 3 人の処理が終わって人間手番へ戻ったら、手牌 13 枚を検知して `drawTile()` を自動実行
- `web/src/game/table.ts`: PixiJS で選択牌の浮き上がり表現を追加。`行動` パネルに最小 action area を実装し、「打牌」ボタンと `canRiichi()` 時のみ「立直」ボタンを表示
- テスト追加: 自動ツモ、タップ選択、再タップ打牌、CPU ループ後の人間復帰を `App.test.ts` で固定

## 残 Issue

| Issue | 内容 | 主要成果物 |
|---|---|---|
| #7 | CPU ターン演出 / ログ / 間 | `WasmGameBridge.executeCpuTurn` の可視化 |
| #8 | 和了画面 / 流局表示 | `web/src/game/scenes/ResultScene.ts` |
| #9 | タイトルシーン + モード選択 | `web/src/game/scenes/TitleScene.ts` |

## 設計メモ

### CUI 表記との整合

牌の Wasm 受け渡しは CUI 表記 (`5mr`, `to`, `hk` 等) を介する。TS 側 `tileToCuiCode` / `tileFromCuiCode` と Rust 側 `Tile::to_string` / `Tile::from_string` は、以下のテーブルで一致させる:

| 種類 | 表記例 |
|---|---|
| 数牌 | `1m`〜`9m` / `1p`〜`9p` / `1s`〜`9s` |
| 赤ドラ | `5mr` / `5pr` / `5sr` |
| 風牌 | `to` (東) / `na` (南) / `sa` (西) / `pe` (北) |
| 三元牌 | `hk` (白) / `ht` (發) / `cn` (中) |

### テスト戦略

jsdom + PixiJS v8 では WebGL は起動できないが、Container / Graphics / Text の生成と階層構造は検証可能。レンダリング系のテストは:

- `App.test.ts`: `Application.stage` を `Container` で差し替えてシーン追加だけ検証
- `tile.test.ts`: `Container.children` を走査して構造・文字色を検証
- `wasm.test.ts`: `__setWasmModuleForTest` で WasmGame を差し替えてラッパ呼び出しを検証

WebGL 込みの E2E は Playwright を別 Issue で導入予定。

### DESIGN.md との関係

`DESIGN.md` は旧 HTML プロトタイプ前提の記述が残っていたため、PixiJS 初期卓に合わせて更新した。現在の扱いは以下:

- ビットマップアセットは使わない
- 牌は Graphics (角丸長方形) + Text glyph で構成
- 戦闘卓は dark felt + brass + crimson vignette
- gradient / glass は loading・title・result など DOM overlay で使う

## CI / 開発フロー

- `web/` 配下で `npm run typecheck && lint && test && build` がすべて通ること
- ルート Rust 側 (`src/*.rs`) は Wasm シグネチャ変更を伴わない限り Web 版作業では触らない
- `web/pkg/` は git 管理外。CI でも `build-wasm.sh` で生成
- 旧 HTML (`web/legacy/`) はビルド対象外、新規開発はしない
