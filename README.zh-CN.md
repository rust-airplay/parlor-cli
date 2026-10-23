# parlor-cli

`parlor`：命令行上的 AirPlay 接收端。

它是程序，不是协议。接收端本身是 [`rust-airplay`](https://github.com/rust-airplay/rust-airplay)，播放由 [`rust-airplay-gstream`](https://github.com/rust-airplay/rust-airplay-gstream) 负责；这个 crate 只解析参数、在磁盘上保存两个文件、安装 logger。

## 构建

```sh
cargo build --release --features gst
```

不带 `--features gst` 时，程序仍可用 `--render null` 构建并运行——读取会话但不播放。带上它则需要 GStreamer：构建要开发库，播放要运行时与插件。各平台的元素清单见后端仓库的 README。

## 运行

```sh
parlor --name "Living room" --port 7000 --render gst
```

在 iPhone 或 iPad 上打开 AirPlay 菜单，选中接收端，输入程序打印的 PIN。客户端一连上就开始镜像；客户端开始播放时就有声音。

## 参数

| 参数 | 含义 |
| --- | --- |
| `--name <name>` | 客户端显示的名字，默认 `parlor` |
| `--port <port>` | 控制端口，默认 7000；`0` 表示自动挑一个 |
| `--pin [<digits>]` | 要求 PIN，可指定固定值；这是默认行为 |
| `--no-pin` | 不要求 PIN |
| `--password <password>` | `SETUP` 时额外要求 digest 口令 |
| `--device-id <address>` | 广播指定的硬件地址 |
| `--seed <hex>` | 32 字节身份，写成 64 个十六进制字符 |
| `--log <file>` | 把请求轨迹追加到文件 |
| `--render <null\|gst>` | 由什么播放媒体，默认 `null` |
| `--audio-sink <name>` | GStreamer 音频 sink |
| `--video-sink <name>` | GStreamer 视频 sink；换它也会换解码器 |
| `--video-parser <name>` | 镜像管线使用的 parser |
| `--video-decoder <name>` | 其后的解码器 |
| `--warm-video` | 视频管线随接收端一起启动 |
| `--pace-video` | 按采集间隔节流视频，画面更均匀但延迟更大 |
| `--width <pixels>` / `--height <pixels>` | 提供给客户端的显示尺寸，默认 1920x1080 |
| `--refresh-rate <hz>` | 提供的刷新率，默认 60 |
| `--max-fps <fps>` | 要求客户端的最高帧率，默认 30 |
| `--overscanned` | 提供 overscan 显示 |
| `--hevc` | 允许 HEVC 并提供 4K，即上游 `-h265` |
| `--features <hex>` | 要广播的 feature 位图 |
| `--restrict` | 只允许用 `--allow` 指定的设备 |
| `--allow <deviceID>` | 允许连接的设备，可重复 |
| `--block <deviceID>` | 永远拒绝的设备，可重复 |
| `--legacy-ports` | 绑定上游的固定媒体端口布局 |
| `--timing-port` / `--udp-control-port` / `--data-port` / `--mirror-port` | 固定某一个媒体端口 |
| `--nohold` | 新客户端顶替当前连接 |
| `--reset <seconds>` | 无 `/feedback` 超过 n 秒关闭会话；`0` 表示从不 |
| `--audio-latency <seconds>` | 客户端排程所用的音频延迟，默认 0.25 秒 |
| `--volume <0..1>` | 会话起始音量，默认满音量 |
| `--taper` | 使用锥形音量曲线，即上游 `-taper` |
| `--db <low:high>` | 把音量滑块重映射到该分贝范围，默认 `-30:0` |
| `--fps-data` | 记录客户端自己的流报告，即上游 `-FPSdata` |
| `--cover-art [file]` | 报告封面图信息，或写入文件；上游 `-ca` |
| `--metadata <file>` | 把当前播放信息写入文件；上游 `-md` |
| `--dacp <file>` | 导出遥控标识；上游 `-dacp` |
| `--audio-codec <alac\|aac-eld>` | 客户端发送的编码，默认 `aac-eld` |
| `--help` | 打印用法并退出 |

程序在本地应用数据目录保存两个文件：`parlor/identity.seed` 与 `parlor/clients`。前者首次运行时创建，是接收端身份保持稳定的原因；后者每行一个 64 字符的十六进制客户端密钥。删除身份文件会让所有客户端重新配对；删除客户端列表则保留身份、清空配对记录。

## 延迟

投屏看的是画面跟手的快慢。最快的路径是硬件解码，在支持的平台上是默认值：Windows 上 `d3d11h264dec` 直接进 `d3d11videosink`。`--video-sink d3d11videosink` 会把这个解码器一起带上；分别指定 sink 与解码器，正是"硬件 sink 被软件解码器喂着"的由来。

## 许可证

GPL-3.0-or-later。见 [LICENSE](LICENSE)。

English: [README.md](README.md)。
