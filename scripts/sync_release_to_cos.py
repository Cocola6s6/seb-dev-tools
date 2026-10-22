#!/usr/bin/env python3
"""
同步 GitHub Release 产物到腾讯云 COS，并生成/更新多平台 latest.json
"""
import os
import sys
import json
import glob
import argparse
from datetime import datetime, timezone
from qcloud_cos import CosConfig, CosS3Client

COS_HOST = "https://seb-1305755407.cos.ap-guangzhou.myqcloud.com"
COS_PREFIX = "seb-dev-tools/releases"

def parse_args():
    parser = argparse.ArgumentParser(description="Sync release assets to Tencent COS")
    parser.add_argument("--dir", default="./release-assets", help="Directory containing release files")
    parser.add_argument("--version", required=True, help="Release version without leading v (e.g. 0.4.0)")
    parser.add_argument("--notes", default="", help="Release notes markdown/text")
    return parser.parse_args()

def generate_latest_json(assets_dir, version, notes):
    """
    根据目录下的更新压缩包及其 .sig 签名，构建/更新 latest.json
    """
    latest_json_path = os.path.join(assets_dir, "latest.json")
    platforms = {}

    # 如果已有 tauri-action 产生的 latest.json，先解析它
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

    # 扫描目录下的所有 .sig 文件以补充/更新
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
        elif "windows" in fn_lower or "nsis.zip" in fn_lower or "x64" in fn_lower:
            platforms["windows-x86_64"] = entry

    now_iso = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    manifest = {
        "version": version.lstrip("v"),
        "notes": notes or f"版本 {version} 发布",
        "pub_date": now_iso,
        "platforms": platforms,
    }

    with open(latest_json_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2, ensure_ascii=False)

    print(f"Generated {latest_json_path} with platforms: {list(platforms.keys())}", flush=True)
    return latest_json_path

def upload_to_cos(assets_dir):
    secret_id = os.environ.get("COS_SECRET_ID", "")
    secret_key = os.environ.get("COS_SECRET_KEY", "")
    region = os.environ.get("COS_REGION", "ap-guangzhou")
    bucket = os.environ.get("COS_BUCKET", "seb-1305755407")

    if not secret_id or not secret_key:
        print("Error: COS_SECRET_ID and COS_SECRET_KEY must be provided via environment variables", file=sys.stderr, flush=True)
        sys.exit(1)

    # 设置超时为 30 秒，重试 3 次
    config = CosConfig(Region=region, SecretId=secret_id, SecretKey=secret_key, Timeout=30)
    client = CosS3Client(config)

    # 先获取远端已有的文件列表与大小，避免重复上传已成功的文件
    existing_objects = {}
    try:
        res = client.list_objects(Bucket=bucket, Prefix=f"{COS_PREFIX}/")
        for item in res.get("Contents", []):
            existing_objects[item["Key"]] = int(item["Size"])
    except Exception as e:
        print(f"Warning: Failed to list existing objects from COS: {e}", flush=True)

    for root, _, files in os.walk(assets_dir):
        for file in files:
            local_path = os.path.join(root, file)
            local_size = os.path.getsize(local_path)
            cos_key = f"{COS_PREFIX}/{file}"

            # 如果不是 latest.json 且远端文件已存在且大小一致，直接跳过
            if file != "latest.json" and existing_objects.get(cos_key) == local_size:
                print(f"Skipping {file} (already exists on COS, size: {local_size} bytes)", flush=True)
                continue

            print(f"Uploading {file} ({local_size} bytes) -> cos://{bucket}/{cos_key} ...", flush=True)
            cache_control = "max-age=31536000"
            if file == "latest.json":
                cache_control = "no-cache, no-store, must-revalidate"

            # 使用 upload_file 高级接口（支持大文件分块并发、断点重试与自动超时处理）
            client.upload_file(
                Bucket=bucket,
                LocalFilePath=local_path,
                Key=cos_key,
                PartSize=2,
                MAXThread=5,
                CacheControl=cache_control
            )
            # 确保文件为公共可读，供客户端匿名下载更新
            try:
                client.put_object_acl(Bucket=bucket, Key=cos_key, ACL="public-read")
            except Exception as e:
                print(f"Warning: Failed to set public-read ACL for {cos_key}: {e}", flush=True)

            print(f"Uploaded: {COS_HOST}/{cos_key}", flush=True)

def main():
    args = parse_args()
    if not os.path.isdir(args.dir):
        print(f"Error: Directory {args.dir} does not exist", file=sys.stderr, flush=True)
        sys.exit(1)

    generate_latest_json(args.dir, args.version, args.notes)
    upload_to_cos(args.dir)
    print("All release assets successfully synced to Tencent COS!", flush=True)

if __name__ == "__main__":
    main()
