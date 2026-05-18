# peerjs-server-rs

🚀 PeerJS Server 的 Rust 实现 - 高性能 WebRTC 信令服务器

## 📖 项目简介

**peerjs-server-rs** 是 [PeerJS](https://github.com/peers/peerjs) 官方 Node.js 服务器的 Rust 语言实现。它提供 WebSocket 信令服务，使得浏览器之间可以通过 P2P 方式建立连接，无需中央服务器中转媒体数据。

### 原作者说明

> **原作者的 README 内容：**
> 
> peerjs server 本质上是一个 ws 通过交换2个 sdp
> 
> 信令服务器收集候选的 ip 列表(询问 tun 服务器知道自己公网 ip,内网 ip)

### 核心功能

- **WebSocket 信令服务**：支持客户端通过 WebSocket 进行 SDP 交换
- **ICE 候选收集**：协助客户端收集公网和内网 IP 地址
- **RESTful API**：提供 `/peerjs/id` 和 `/peerjs/peers` 端点
- **多平台支持**：支持 x86_64、ARM 等多种架构
- **配置文件支持**：支持 TOML 格式配置文件

## 🛠️ 技术栈

- **开发语言**：Rust
- **Web 框架**：Axum
- **WebSocket**：tokio-tungstenite
- **HTTP 中间件**：Tower HTTP
- **命令行解析**：Clap

## 📦 功能对比

### 与官方 peerjs-server (Node.js) 对比

| 功能 | 官方 peerjs-server (Node.js) | peerjs-server-rs (Rust) |
|------|-----------------------------|-------------------------|
| WebSocket 信令 | ✅ | ✅ |
| REST API (`/peerjs/id`) | ✅ | ✅ |
| REST API (`/peerjs/peers`) | ✅ | ✅ |
| 消息队列 | ✅ | ✅ |
| 心跳检测 | ✅ | 🔄 开发中 |
| 连接超时检查 | ✅ | 🔄 开发中 |
| Key 认证 | ✅ | ✅ |
| 并发连接限制 | ✅ | ✅ |
| **HTTPS/TLS 原生支持** | ❌ 需反向代理 | ✅ **直接支持** |
| 跨平台编译 | ⚠️ 需 Node.js 环境 | ✅ GitHub Actions |
| Docker 支持 | ✅ | ❌ 暂不支持 |

### 与原分叉项目对比

本项目是在原 Rust 实现基础上进行了大量增强，主要改进包括：

| 功能 | 原分叉 | peerjs-server-rs |
|------|--------|------------------|
| 命令行端口配置 | ❌ 硬编码 | ✅ `-p/--port` 参数 |
| 配置文件支持 | ❌ 无 | ✅ TOML 格式，自动生成 |
| TLS/HTTPS 支持 | ❌ 无 | ✅ 原生支持 |
| 日志系统 | ❌ 简单 println | ✅ Tracing 结构化日志 |
| 配置版本管理 | ❌ 无 | ✅ 自动迁移与备份 |
| 多平台编译 | ❌ 需手动 | ✅ GitHub Actions |

### 主要增强功能

1. **原生 HTTPS/TLS 支持**：无需反向代理即可启用 HTTPS/WSS，提供更安全的连接
2. **配置文件管理**：支持 TOML 配置，自动版本检测与迁移，旧配置自动备份
3. **灵活的日志系统**：5 级日志级别，支持请求头调试
4. **命令行参数**：支持自定义端口和配置文件路径

## 📦 下载与安装

### 从 Releases 下载（推荐）

访问 [GitHub Releases 页面](https://github.com/loginyourheart/SignalServer/releases) 下载对应平台的预编译二进制文件：

- **Windows:** `.zip` 文件，直接解压即可使用
- **Linux/macOS:** `.tar.gz` 文件，解压后 `chmod +x peerjs-server-rs` 添加执行权限

### 编译

```bash
# 克隆项目
git clone https://github.com/loginyourheart/SignalServer.git
cd SignalServer

# 编译
cargo build --release
```

### 运行

**Linux/macOS:**
```bash
# 使用默认配置（端口 9000，配置文件 config.toml）
./target/release/peerjs-server-rs

# 指定端口
./target/release/peerjs-server-rs -p 8080

# 指定配置文件
./target/release/peerjs-server-rs -c /path/to/config.toml
```

**Windows:**
```cmd
# 使用默认配置
.\target\release\peerjs-server-rs.exe

# 指定端口
.\target\release\peerjs-server-rs.exe -p 8080

# 指定配置文件
.\target\release\peerjs-server-rs.exe -c C:\path\to\config.toml
```

## ⚙️ 配置文件

首次运行时，程序会自动在当前目录生成 `config.toml` 配置文件：

```toml
# ========================================
# PeerJS Server Rust - 配置文件
# ========================================
# 配置文件版本: 1.2.0
# ========================================

# --------------------------
# 基本配置
# --------------------------

# 配置文件版本（请勿手动修改）
config_version = "1.2.0"

# PeerJS 认证密钥（客户端连接时需要提供相同的 key）
key = "peerjs"

# 最大并发连接数
concurrent_limit = 5000

# API 路由路径
path = "/peerjs"

# 是否允许列出所有在线客户端（listAllPeers）
allow_discovery = false

# --------------------------
# 健康检查配置
# --------------------------

# 客户端存活超时（毫秒）
alive_timeout = 60000

# 连接检查间隔（秒）
check_interval = 300

# 清理过期消息间隔（毫秒）
cleanup_out_msgs = 10000

# 消息过期时间（毫秒）
expire_timeout = 5000

# --------------------------
# TLS/HTTPS 配置
# --------------------------

# 是否启用 TLS（HTTPS/WSS）
tls_enabled = false

# TLS 证书文件路径（示例: /etc/ssl/certs/server.crt 或 ./certs/fullchain.pem）
tls_cert_path = "./certs/fullchain.pem"

# TLS 私钥文件路径（示例: /etc/ssl/private/server.key 或 ./certs/privkey.pem）
tls_key_path = "./certs/privkey.pem"

# --------------------------
# 日志配置
# --------------------------

# 日志级别: trace, debug, info, warn, error
log_level = "info"

# 是否打印请求头调试信息
debug_request_headers = false
```

### 配置文件版本管理

- 配置文件包含 `config_version` 字段用于版本检测
- 当程序检测到旧版本配置时，会自动：
  1. 备份旧配置（命名为 `config.toml.backup.YYYYMMDDHHMMSS`）
  2. 保留旧配置中的自定义值
  3. 生成包含所有新配置项的新配置文件
- 无需手动修改，程序会自动处理迁移

### 完整配置项说明

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| **基本配置** | | |
| `config_version` | `"1.2.0"` | 配置文件版本（自动管理，请勿手动修改） |
| `key` | `"peerjs"` | PeerJS 认证密钥，客户端连接时需要提供 |
| `concurrent_limit` | `5000` | 最大并发连接数 |
| `path` | `"/peerjs"` | API 路由路径 |
| `allow_discovery` | `false` | 是否允许列出所有在线 Peer（`listAllPeers()` 功能） |
| **健康检查配置** | | |
| `alive_timeout` | `60000` | 客户端存活超时时间（毫秒） |
| `check_interval` | `300` | 连接检查间隔（秒） |
| `cleanup_out_msgs` | `10000` | 消息清理任务执行间隔（毫秒） |
| `expire_timeout` | `5000` | 消息过期时间（毫秒） |
| **TLS/HTTPS 配置** | | |
| `tls_enabled` | `false` | 是否启用 TLS（HTTPS/WSS），设为 `true` 后需配置证书 |
| `tls_cert_path` | `"cert.pem"` | TLS 证书文件路径 |
| `tls_key_path` | `"key.pem"` | TLS 私钥文件路径 |
| **日志配置** | | |
| `log_level` | `"info"` | 日志级别：`trace`, `debug`, `info`, `warn`, `error` |
| `debug_request_headers` | `false` | 是否在日志中打印请求头信息（调试用） |

### 命令行参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `-p, --port` | `9000` | 服务器监听端口 |
| `-c, --config` | `config.toml` | 配置文件路径 |

## 🌐 支持的平台

本项目支持多种平台，所有平台的预编译二进制文件均可从 Releases 页面下载：

| 平台 | 架构 | 说明 |
|------|------|------|
| Linux x86_64 | glibc | 标准 Linux 服务器 |
| Linux x86_64 | musl | Alpine Linux 等轻量级发行版 |
| ARM v7 | glibc | 树莓派 3B/4 (Raspbian/Ubuntu) |
| ARM v7 | musl | 树莓派 3B + ImmortalWrt |
| ARM64 | glibc | 树莓派 4/5 64位 |
| ARM64 | musl | ARM64 musl 系统 |
| ARM v6 | glibc | 树莓派 Zero/1 |
| Windows x86_64 | - | Windows 64位 |
| Windows x86 | - | Windows 32位 |
| macOS x86_64 | - | macOS Intel |
| macOS ARM64 | - | macOS Apple Silicon (M1/M2/M3) |

## 📚 工作原理

PeerJS 服务器的核心任务是协助两个浏览器客户端建立 P2P 连接：

1. **SDP 交换**：两个客户端各自生成 SDP（Session Description Protocol）offer/answer
2. **信令传输**：通过 WebSocket 将 SDP 从一方传递给另一方
3. **ICE 候选**：收集候选的 IP 和端口信息（包括公网和内网）
4. **连接建立**：客户端使用交换的 SDP 和 ICE 候选直接建立 P2P 连接

## 📋 更新日志

### v1.2.0 (2026-05-18)

**✨ 新功能**
- 新增配置文件版本管理系统，支持自动检测和迁移
- 旧配置文件自动备份功能（时间戳命名）
- 配置文件包含完整的 `config_version` 字段

**🔧 改进**
- 优化配置文件加载逻辑，向后兼容旧版本
- 配置文件生成包含完整的所有配置项（包括新增项）
- 启动时显示配置版本信息

### v1.1.0 (2026-05-17)

**✨ 新功能**
- 新增日志级别配置，支持 `trace`/`debug`/`info`/`warn`/`error` 五个等级
- 新增请求头调试开关 `debug_request_headers`
- 配置文件自动生成带有详细中文注释
- 配置文件结构更清晰，按功能分类

**🔧 改进**
- 优化日志系统初始化逻辑
- 向后兼容旧版本配置文件（新增字段有默认值）

### v1.0.0 (2026-05-16)

**✨ 核心功能**
- ✅ 原生 TLS/HTTPS 支持，无需反向代理即可使用 WSS
- ✅ HTTP/2 反向代理 WebSocket 兼容中间件（解决 Lucky 等代理问题）
- ✅ 完整的配置文件支持（TOML 格式）
- ✅ 多平台编译支持（Windows、macOS、Linux、ARM 系列）
- ✅ 命令行端口配置
- ✅ 绑定地址 0.0.0.0 以支持外部访问

**🏗️ 技术架构**
- 使用 Axum + Tokio 异步运行时
- Rustls 提供 TLS 支持
- Tracing 结构化日志系统
- 支持 ARM v6/v7/ARM64 等嵌入式平台

**📦 平台支持**
- Linux x86_64 (glibc/musl)
- Windows x86/x86_64
- macOS (Intel/Apple Silicon)
- ARM v6/v7/ARM64（树莓派全系列支持）
- ImmortalWrt (musl) 等轻量级 Linux 系统

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

本项目遵循 MIT 许可证。

## 🔗 相关链接

- [PeerJS 官方库](https://github.com/peers/peerjs)
- [官方 peerjs-server](https://github.com/peers/peerjs-server)
- [Axum Web 框架](https://github.com/tokio-rs/axum)
