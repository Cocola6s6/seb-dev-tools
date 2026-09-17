#!/bin/bash
# 启动 Rust 版（Tauri + Sycamore）开发模式。
set -e
cd "$(dirname "$0")/crates/seb-app"

for bin in cargo trunk; do
    if ! command -v "$bin" >/dev/null 2>&1; then
        echo "缺少 $bin，请先执行："
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo "  rustup target add wasm32-unknown-unknown"
        echo "  cargo install trunk tauri-cli --locked"
        exit 1
    fi
done

echo "正在启动中控调试与 ECU 配置发包工具（Rust 版）..."
cargo tauri dev
