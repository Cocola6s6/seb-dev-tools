# seb-dev-tools（共享单车内网调试工具）

Tauri 2 + Sycamore 0.9（WASM）桌面工具：一键把车辆接入内网环境，并下发中控指令与中控配置。

## 一、结构介绍

```
seb-dev-tools/
├── Cargo.toml                  # workspace：seb-core + seb-app
├── run_dev.sh                  # 开发模式启动脚本
└── crates/
    ├── seb-core/               # 核心库（原生），不认识 Tauri 与界面
    │   ├── assets/ecu_params_dict.json   # 86 项 ECU 参数，编译期内嵌
    │   └── src/
    │       ├── config.rs       # MQ / MySQL 连接常量、本地配置读写
    │       ├── mq.rs           # RabbitMQ 发布端，断线自动重连重发
    │       ├── db.rs           # 一键接入：直连 MySQL 写车辆与绑定关系
    │       ├── redis.rs        # 手写 RESP 客户端：查实例号、写上线鉴权缓存
    │       ├── payload.rs      # 下行 JSON 报文构造
    │       ├── ecu.rs          # ECU 参数字典与头盔一键场景
    │       └── frame.rs / device.rs      # TBIT 二进制报文编解码与设备直连链路
    └── seb-app/
        ├── src/                # 原生壳：窗口启动 + #[tauri::command] 命令层
        │   ├── main.rs / lib.rs
        │   └── commands/{config,instance,deploy,ecu,send}.rs
        ├── tauri.conf.json     # 窗口、打包、前端构建配置
        └── ui/                 # 前端：Sycamore 界面，编译目标 wasm32
            ├── index.html      # 页面骨架与样式
            └── src/
                ├── main.rs     # 入口、侧边导航
                ├── api.rs      # invoke 调用桥
                ├── state.rs    # 全局上下文、日志、与后端对齐的数据结构
                ├── components.rs
                └── pages/{deploy,control,ecu}.rs
```

**为什么 `seb-app` 下面还有 `ui`**

Tauri 的模型是「原生壳子 + 内嵌 WebView」，一个 app 里其实是两个编译目标完全不同的程序：

| 位置 | 编译目标 | 职责 |
| --- | --- | --- |
| `crates/seb-core` | 原生 lib | 纯业务逻辑，可脱离 GUI 复用 |
| `crates/seb-app/src` | 原生 bin | 薄封装：把核心能力注册成 Tauri 命令，持有全局 `Publisher` |
| `crates/seb-app/ui` | wasm bin | 界面，只负责 invoke，不碰任何网络 |

窗口里显示的是网页，所以 `ui/` 必须编成 wasm 跑在 WebView 里；而 RabbitMQ、MySQL、Redis 这些
连接必须是原生代码——wasm 在浏览器沙箱里没有 TCP socket，`sqlx` / `amqprs` 根本编译不过去。
两边靠 Tauri IPC 通信：前端 `api.rs` 里的 `invoke("bike_deploy", …)` 对应后端
`commands/deploy.rs` 上标了 `#[tauri::command]` 的函数，`ui/src/state.rs` 里的结构体是
seb-core 同名类型的镜像，两者不共享代码，只共享 serde 的 camelCase JSON 格式。

`ui/Cargo.toml` 末尾有一个空的 `[workspace]` 表，作用是把 ui 从上级工作区摘出去成为独立工作区，
否则 cargo 会试图把 sycamore/web-sys 按原生目标编译、又把 sqlx/tauri 拖进 wasm 目标。代价是
ui 有自己的 `Cargo.lock` 与 `target/`，编译检查要分两次跑：

```bash
cargo check -p seb-core -p seb-app
```

```bash
cd crates/seb-app/ui && cargo check --target wasm32-unknown-unknown
```

连接信息（RabbitMQ / MySQL / Redis）全部写死在 `seb-core/src/config.rs` 与 `redis.rs`，界面上
不需要配任何连接参数；车辆相关的默认值保存在 `~/.config/seb-dev-tools/config.json`。

## 二、启动说明

### 环境准备

需要 rustc / cargo、`wasm32-unknown-unknown` 目标、trunk、tauri-cli。换机器时执行：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

```bash
rustup target add wasm32-unknown-unknown && cargo install trunk tauri-cli --locked
```

rustup 装在 `~/.cargo/bin`，若当前 shell 找不到命令：

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### 开发模式

```bash
./run_dev.sh
```

脚本会先检查 cargo / trunk 是否就绪，再进入 `crates/seb-app` 执行 `cargo tauri dev`。也可以手动跑：

```bash
cd crates/seb-app && cargo tauri dev
```

### 打包

```bash
cd crates/seb-app && cargo tauri build
```

`tauri.conf.json` 的 `beforeDevCommand` / `beforeBuildCommand` 会自动在 `ui/` 目录执行 trunk
（开发模式起 127.0.0.1:1420 的热重载服务，打包时产出 `ui/dist/`），**不需要手动启动前端**。
首次启动要编译全部依赖，耗时较长；之后增量编译很快。
