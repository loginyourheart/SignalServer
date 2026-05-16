peerjs server 本质上是一个ws 通过交换2个sdp
信令服务器收集候选的ip列表(询问tun服务器知道自己公网ip,内网ip)

## 使用教程

### 编译

```bash
cargo build --release
```

### 运行

```bash
# 使用默认端口 9000
./target/release/SignalServer

# 使用自定义端口
./target/release/SignalServer -p 8080
./target/release/SignalServer --port 8080
```

### GitHub Actions 多平台构建

本项目支持通过 GitHub Actions 为多种平台编译二进制文件：

| 平台 | 架构 | 说明 |
|------|------|------|
| Linux x86_64 | glibc | 标准 Linux 服务器 |
| Linux x86_64 | musl | Alpine Linux 等 |
| ARM v7 | glibc | 树莓派 3B/4 (glibc) |
| ARM v7 | musl | **树莓派 3B + ImmortalWrt** (你的目标) |
| ARM64 | glibc | 树莓派 4/5 64位 |
| ARM64 | musl | ARM64 musl 系统 |
| ARM v6 | glibc | 树莓派 Zero/1 |

访问 Actions 页面下载对应平台的二进制文件：
https://github.com/loginyourheart/SignalServer/actions

### Docker 部署示例

```bash
docker run -d -p 9000:9000 signalserver
```
