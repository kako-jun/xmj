#!/bin/bash

# WebAssemblyビルドスクリプト

echo "Building xmj for WebAssembly..."

# wasm-packがインストールされているか確認
if ! command -v wasm-pack &> /dev/null; then
    echo "Error: wasm-pack is not installed"
    echo "Install it with: cargo install wasm-pack"
    exit 1
fi

# WASMビルド
wasm-pack build --target web --features wasm

# 出力ディレクトリを移動
if [ -d "pkg" ]; then
    rm -rf web/pkg
    mv pkg web/pkg
fi

echo "✅ WASM build completed!"
echo "Output: web/pkg/"
