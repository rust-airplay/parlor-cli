# parlor-cli

[![CI](https://github.com/rust-airplay/parlor-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/rust-airplay/parlor-cli/actions/workflows/ci.yml)

`parlor`：命令行上的 AirPlay 接收端。

它是程序，不是协议。接收端本身是 [`rust-airplay`](https://github.com/rust-airplay/rust-airplay)，播放由 [`rust-airplay-gstream`](https://github.com/rust-airplay/rust-airplay-gstream) 负责；这个 crate 解析参数、保存两个文件、安装 logger 并打印。它不含任何协议代码。

## 构建

```sh
cargo build --release --features gst
```

不带 `--features gst` 时，程序仍能构建，但不播放：它读取会话并报告，这是在无声卡的机器上检查会话的方式。带上该 feature 则需要安装 GStreamer——构建要开发文件，运行要运行时与插件。各平台所需的元素见后端仓库的 README。

## 运行

```sh
parlor -n "Living room" -pin 1234
```

带 `gst` feature 的构建默认通过 GStreamer 播放；`--render null` 用于只读会话、不放声音的场景。

程序打印一段简短标题后等待：

```
parlor 0.2.0 - an AirPlay receiver

  Name      Living Room
  Port      7000
  Playback  GStreamer
  Clients   0 remembered

Waiting for a client. Press Ctrl+C to stop.
```

在 iPhone 或 iPad 上打开 AirPlay 菜单，选中接收端，在提示出现时输入 PIN。客户端一连上就开始镜像；客户端开始播放时就有声音。

## 控制台

标题只打印一次。之后控制台只显示警告与错误；PIN 或生成的口令在需要时打印，与日志级别无关。

| 参数 | 显示内容 |
| --- | --- |
| `-q` | 只有错误 |
| 默认 | 警告与错误 |
| `-v`、`--verbose` | 再加上会话记录：流打开、时钟、媒体统计 |
| `-d` | 再加上协议细节 |
| `--log-level <level>` | 直接指定：`off`、`error`、`warn`、`info`、`debug`、`trace` |

`--log <file>` 把请求轨迹追加到文件，与以上级别无关。

## 参数

| 参数 | 含义 |
| --- | --- |
| `-n <name>` | 客户端显示的名字，默认 `parlor` |
| `-nh` | 接受；本接收端从不在名字后附加主机名 |
| `-h265` | 允许 HEVC 并提供 4K |
| `-pin [nnnn]` | 要求 PIN，可指定固定值；这是默认行为 |
| `-pw [password]` | 要求口令；不给口令时随机生成并打印 |
| `-m [mac]` | 广播指定的硬件地址 |
| `-key [file]` | 身份种子存放位置；不给文件则完全不保存 |
| `-reg [file]` | 已记住客户端的存放位置 |
| `-p [n]` | 固定媒体端口布局，或基准端口 `n` |
| `-al <seconds>` | 客户端排程所用的音频延迟，默认 0.25 秒 |
| `-vol <0..1>` | 会话起始音量 |
| `-db <low:high>` | 把音量滑块重映射到该分贝范围，默认 `-30:0` |
| `-taper` | 使用锥形音量曲线 |
| `-s <w>x<h>[@<r>]` | 提供给客户端的显示，默认 1920x1080@60 |
| `-fps <fps>` | 要求客户端的最高帧率，默认 30 |
| `-o` | 提供 overscan 显示 |
| `-as <sink>` / `-vs <sink>` | GStreamer 音频与视频 sink |
| `-vp <parser>` / `-vd <decoder>` | 镜像管线使用的 parser 与解码器 |
| `-avdec` | 强制使用软件解码器 |
| `-ca [file]` | 报告封面图信息，或写入文件 |
| `-md <file>` | 把当前播放信息写入文件 |
| `-dacp [file]` | 导出遥控标识 |
| `-reset <seconds>` | 无 `/feedback` 超过 n 秒关闭会话；`0` 表示从不 |
| `-nohold` | 新客户端顶替当前连接 |
| `-restrict` / `-allow <id>` / `-block <id>` | 允许哪些设备连接 |
| `-FPSdata` | 记录客户端自己的流报告 |
| `-q`、`-v`、`-d`、`--log-level` | 控制台显示什么 |
| `-h`、`-help` | 打印用法并退出 |

每个参数也都有长名字，给偏好单词的脚本：`--name`、`--port`、`--no-pin`、`--password`、`--device-id`、`--seed`、`--log`、`--render`、`--audio-sink`、`--video-sink`、`--video-parser`、`--video-decoder`、`--warm-video`、`--pace-video`、`--width`、`--height`、`--refresh-rate`、`--max-fps`、`--overscanned`、`--hevc`、`--features`、`--restrict`、`--allow`、`--block`、`--legacy-ports`、`--timing-port`、`--udp-control-port`、`--data-port`、`--mirror-port`、`--nohold`、`--reset`、`--audio-latency`、`--volume`、`--taper`、`--db`、`--fps-data`、`--cover-art`、`--metadata`、`--dacp`、`--audio-codec`、`--key-file`、`--clients-file`、`--ephemeral-seed`、`--random-password`、`--verbose`、`--base-port`。

### 端口布局与口令

- `-p n` 按 `tcp`/`udp` 占 `n..n+2` 布置端口，控制端口是 `n+1`。只写 `-p` 选择固定布局：镜像 `7100`、控制 `7000`、timing `7011`、UDP 控制 `6001`、数据 `6000`。
- `-pw` 不带口令时，本程序为整个进程生成一次并打印。

## 文件

程序在本地应用数据目录下保存两个文件：`parlor/identity.seed` 与 `parlor/clients`。

- `identity.seed` 首次运行时生成，是接收端跨重启保持可识别的原因。删除它会让所有客户端重新配对。
- `clients` 每行一个 64 字符十六进制客户端密钥。删除它保留身份、只忘记客户端。

`-key` 与 `-reg` 可以搬动这两个文件；`-key` 不带文件表示身份只存在于内存里，什么都不保存。

## 延迟

投屏看的是画面跟手的快慢。最快的路径是硬件路径，在有它的平台上就是默认：Windows 上的 `d3d11h264dec` 进 `d3d11videosink`。`-vs d3d11videosink` 会把解码器一起换过去；单独指定 sink 与解码器，会让硬件 sink 被软件解码器喂数据。

## 开发

```sh
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets --features gst -- -D warnings
```

测试解析命令行：参数名、端口布局、可选值、以及对无效值的拒绝。它们不需要 GStreamer。

## 发布

`Release` 工作流手动触发，有三个输入：形如 `v0.2.0` 的 tag、发布说明（换行用 `
`）、以及是否创建草稿。它使用 GStreamer 后端为 Linux 与 Windows 构建 `parlor`，把二进制、两个 README 与许可证打包，并创建 GitHub release。Windows 构建使用与 MSYS2 GStreamer 库匹配的 GNU 工具链；两个压缩包都要求运行机器安装 GStreamer 1.0 及基础插件。

## 文档

- [rust-airplay 的 API](https://docs.rs/rust-airplay)：`Config`、渲染器 trait、各模块
- [rust-airplay 的后端指南](https://github.com/rust-airplay/rust-airplay/blob/main/docs/BACKEND.md)：如何实现播放后端
- [rust-airplay-gstream 的 README](https://github.com/rust-airplay/rust-airplay-gstream)：播放路径的 sink、元素与行为
- [CHANGELOG.md](CHANGELOG.md)：每个版本包含什么

## 许可证

GPL-3.0-or-later。见 [LICENSE](LICENSE)。

English: [README.md](README.md)。
