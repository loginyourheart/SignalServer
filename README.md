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

## 📦 与官方 peerjs-server 功能对比

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
| 跨平台编译 | ⚠️ 需 Node.js 环境 | ✅ GitHub Actions |
| Docker 支持 | ✅ | ❌ 暂不支持 |

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
key = "peerjs"
concurrent_limit = 5000
path = "/peerjs"
allow_discovery = false
alive_timeout = 60000
check_interval = 300
cleanup_out_msgs = 10000
expire_timeout = 5000
```

### 配置项说明

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `key` | `"peerjs"` | PeerJS 认证密钥，客户端连接时需要提供 |
| `concurrent_limit` | `5000` | 最大并发连接数 |
| `path` | `"/peerjs"` | API 路由路径 |
| `allow_discovery` | `false` | 是否允许列出所有在线 Peer（`listAllPeers()` 功能） |
| `alive_timeout` | `60000` | 客户端存活超时时间（毫秒） |
| `check_interval` | `300` | 连接检查间隔（秒） |
| `cleanup_out_msgs` | `10000` | 消息清理任务执行间隔（毫秒） |
| `expire_timeout` | `5000` | 消息过期时间（毫秒） |

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

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

本项目遵循 MIT 许可证。

## 🔗 相关链接

- [PeerJS 官方库](https://github.com/peers/peerjs)
- [官方 peerjs-server](https://github.com/peers/peerjs-server)
- [Axum Web 框架](https://github.com/tokio-rs/axum)
