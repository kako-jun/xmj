// Wasm 連携の集約ポイント。Issue #2 段階では init のみ実装する。
// Issue #3 で WasmGame ラッパ (drawTile/discardTile 等) を追加する。

// pkg は build-wasm.sh で生成される副産物。dev / build / test 前に
// prescript で sync-wasm が走る前提。
//
// 型は pkg/xmj_core.d.ts を tsconfig 経由で参照。

let initialized = false

export const initWasm = async (): Promise<void> => {
  if (initialized) return
  // vite-plugin-wasm が wasm-bindgen 形式 (default export = init function) を扱う。
  const mod = await import('../../pkg/xmj_core.js')
  // wasm-pack --target web の出力は default export として init(url) を持つ。
  // 引数なしで呼ぶと package.json の main 隣の .wasm を fetch する。
  if (typeof mod.default === 'function') {
    await mod.default()
  }
  initialized = true
}

/**
 * Wasm が初期化済みかを返す (テスト用)。
 */
export const isWasmReady = (): boolean => initialized

/**
 * テスト用のリセット。本番コードからは呼ばない。
 */
export const __resetWasmForTest = (): void => {
  initialized = false
}
