#!/usr/bin/env python3
"""
生成/更新多平台 latest.json，适配腾讯云 COS 下载地址（纯 Python 标准库，零第三方依赖）
"""
import os
import sys
import json
import glob
import argparse
from datetime import datetime, timezone

COS_HOST = "https://seb-1305755407.cos.ap-guangzhou.myqcloud.com"
COS_PREFIX = "seb-dev-tools/releases"

def parse_args():
    parser = argparse.ArgumentParser(description="Generate latest.json for Tencent COS")
    parser.add_argument("--dir", default="./release-assets", help="Directory containing release files")
    parser.add_argument("--version", required=True, help="Release version without leading v (e.g. 0.4.2)")
    parser.add_argument("--notes", default="", help="Release notes markdown/text")
    parser.add_argument("--generate-only", action="store_true", help="Kept for backward compatibility")
    return parser.parse_args()

def generate_latest_json(assets_dir, version, notes):
    """
    根据目录下的更新压缩包及其 .sig 签名，构建/更新 latest.json
    """
    latest_json_path = os.path.join(assets_dir, "latest.json")
    platforms = {}

    # 如果已有 tauri-action 产生的 latest.json，先解析它并转换为 COS 地址
    if os.path.exists(latest_json_path):
        try:
            with open(latest_json_path, "r", encoding="utf-8") as f:
                data = json.load(f)
                if isinstance(data.get("platforms"), dict):
                    for plat, info in data["platforms"].items():
                        url = info.get("url", "")
                        filename = os.path.basename(url)
                        info["url"] = f"{COS_HOST}/{COS_PREFIX}/{filename}"
                        platforms[plat] = info
        except Exception as e:
            print(f"Warning: Failed to parse existing latest.json: {e}", flush=True)

    # 扫描目录下的所有 .sig 文件以补充/更新平台签名
    sig_files = glob.glob(os.path.join(assets_dir, "*.sig"))
    for sig_path in sig_files:
        base_asset = sig_path[:-4]  # 去掉 .sig
        if not os.path.exists(base_asset):
            continue

        filename = os.path.basename(base_asset)
        with open(sig_path, "r", encoding="utf-8") as sf:
            signature = sf.read().strip()

        file_url = f"{COS_HOST}/{COS_PREFIX}/{filename}"
        entry = {
            "signature": signature,
            "url": file_url,
        }

        fn_lower = filename.lower()
        if "darwin" in fn_lower or ".app.tar.gz" in fn_lower:
            if "arm64" in fn_lower or "aarch64" in fn_lower:
                platforms["darwin-aarch64"] = entry
            elif "x86_64" in fn_lower:
                platforms["darwin-x86_64"] = entry
            else:
                platforms["darwin-aarch64"] = entry
                platforms["darwin-x86_64"] = entry
                platforms["darwin-aarch64-app"] = entry
                platforms["darwin-x86_64-app"] = entry
        elif "windows" in fn_lower or "nsis.zip" in fn_lower or "msi.zip" in fn_lower or "x64" in fn_lower:
            platforms["windows-x86_64"] = entry
            platforms["windows-x86_64-msi"] = entry

    now_iso = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    manifest = {
        "version": version.lstrip("v"),
        "notes": notes or f"版本 {version} 发布",
        "pub_date": now_iso,
        "platforms": platforms,
    }

    with open(latest_json_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2, ensure_ascii=False)

    print(f"Generated {latest_json_path} successfully! Platforms: {list(platforms.keys())}", flush=True)
    return latest_json_path

def main():
    args = parse_args()
    if not os.path.isdir(args.dir):
        print(f"Error: Directory {args.dir} does not exist", file=sys.stderr, flush=True)
        sys.exit(1)

    generate_latest_json(args.dir, args.version, args.notes)
    print("latest.json generation finished! Ready for coscli sync.", flush=True)

if __name__ == "__main__":
    main()
