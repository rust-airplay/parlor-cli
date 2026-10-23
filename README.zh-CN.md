# parlor-cli

`parlor`：命令行上的 AirPlay 接收端。

它是程序，不是协议。接收端本身是 [`rust-airplay`](https://github.com/rust-airplay/rust-airplay)，播放由 [`rust-airplay-gstream`](https://github.com/rust-airplay/rust-airplay-gstream) 负责；这个 crate 只解析参数、在磁盘上保存两个文件、安装 logger。

## 构建

```sh
cargo build --release --features gst
```

不带 `--features gst` 时，程序只能读会话、不播放（即 `--render null`）。带上它则需要 GStreamer：构建要开发库，播放要运行时与插件。各平台的元素清单见后端仓库的 README。

## 运行

```sh
parlor -n "Living room" -pin 1234
```

带 `gst` feature 的构建默认通过 GStreamer 播放，这正是 UxPlay 命令所期望的；`--render null` 留给"只读会话不放声音"的场景。

在 iPhone 或 iPad 上打开 AirPlay 菜单，选中接收端，输入程序打印的 PIN。客户端一连上就开始镜像；客户端开始播放时就有声音。

## 参数

参数名与 UxPlay 的命令行一致：凡是本接收端能做的，UxPlay 的命令可以原样跑。

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
| `-p [n]` | 上游的固定端口布局，或基准端口 `n` |
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
| `-d` | 日志更详细 |
| `-h`、`-help` | 打印用法并退出 |

本接收端无法兑现的 UxPlay 参数会被**拒绝并说明原因**，而不是被忽略：`-hls` 及其调优参数、`-mp4`、`-p2p`、`-vsync`、`-async`、`-vrtp`、`-artp`、`-v4l2`、`-bt709`、`-srgb`、`-fs`、`-f`、`-r`、`-nc`、`-nofreeze`、`-vc`。需要这些功能的命令必须在用户看得见的地方失败，而不是跑出一场"其实不是所要求"的会话。

每个参数也都有长名字，给偏好单词的脚本：`--name`、`--port`、`--no-pin`、`--password`、`--device-id`、`--seed`、`--log`、`--render`、`--audio-sink`、`--video-sink`、`--video-parser`、`--video-decoder`、`--warm-video`、`--pace-video`、`--width`、`--height`、`--refresh-rate`、`--max-fps`、`--overscanned`、`--hevc`、`--features`、`--restrict`、`--allow`、`--block`、`--legacy-ports`、`--timing-port`、`--udp-control-port`、`--data-port`、`--mirror-port`、`--nohold`、`--reset`、`--audio-latency`、`--volume`、`--taper`、`--db`、`--fps-data`、`--cover-art`、`--metadata`、`--dacp`、`--audio-codec`、`--key-file`、`--clients-file`、`--ephemeral-seed`、`--random-password`、`--verbose`、`--base-port`。

程序在本地应用数据目录下保存两个文件：`parlor/identity.seed` 与 `parlor/clients`。前者首次运行时生成，是接收端身份保持稳定的原因；后者每行一个 64 字符十六进制客户端密钥。删除身份会让所有客户端重新配对；删除客户端列表则保留身份、只忘记客户端。`-key` 与 `-reg` 可以搬动这两个文件，`-key` 不带文件则表示身份只存在于内存里。

## 延迟

投屏看的是画面跟手的快慢。最快的路径是硬件解码，在支持的平台上是默认值：Windows 上 `d3d11h264dec` 直接进 `d3d11videosink`。`--video-sink d3d11videosink` 会把这个解码器一起带上；分别指定 sink 与解码器，正是"硬件 sink 被软件解码器喂着"的由来。

## 许可证

GPL-3.0-or-later。见 [LICENSE](LICENSE)。

English: [README.md](README.md)。
