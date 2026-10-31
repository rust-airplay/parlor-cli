# parlor 使用指南

English: [GUIDE.md](GUIDE.md)。

`parlor` 是 Windows 与 Linux 上的 AirPlay 接收端。它在局域网里广播自己，与 iPhone、iPad 或 Mac
配对，并通过 GStreamer 播放投屏画面与音频。所有讲协议的部分都在
[`rust-airplay`](https://github.com/rust-airplay/rust-airplay) 与
[`rust-airplay-gstream`](https://github.com/rust-airplay/rust-airplay-gstream) 里；`parlor` 是
包在外面的程序：命令行、文件、控制台。

这份指南讲实际操作：怎么安装、怎么把 GStreamer 配好、每个参数做什么、没有画面时该看什么。
更短的 [README](README.md) 是速查表。

**目录**

1. [安装](#1-安装)
2. [GStreamer](#2-gstreamer)
3. [首次运行与配对](#3-首次运行与配对)
4. [日常使用](#4-日常使用)
5. [参数](#5-参数)
6. [常用配方](#6-常用配方)
7. [文件](#7-文件)
8. [退出码](#8-退出码)
9. [故障排查](#9-故障排查)
10. [安全](#10-安全)

## 1. 安装

### 从压缩包安装

发布压缩包包含二进制、README 与许可证：

- Windows：`parlor-v0.3.2-x86_64-pc-windows-gnu.zip`——解压后运行 `parlor.exe`。
- Linux：`parlor-v0.3.2-x86_64-unknown-linux-gnu.tar.gz`——解压后运行 `./parlor`。

两者都用 GStreamer 后端构建，所以 **GStreamer 是前置条件**（第 2 节）。压缩包里不带它。

### 从源码构建

```sh
git clone https://github.com/rust-airplay/parlor-cli
cd parlor-cli
cargo build --release --features gst     # 播放视频与音频
cargo build --release                    # 读取会话，不播放
```

需要 Rust 1.85 或更新。两个库是 `Cargo.toml` 里锁定到某个修订的 git 依赖，所以第一次构建会克隆
它们；之后 `cargo build --offline` 可用。用 `--locked` 可以精确构建发布时测试过的依赖图。

二进制落在 `target/release/parlor`（Windows 上是 `parlor.exe`）。

### 检查构建

```sh
./parlor --version          # parlor 0.3.2
./parlor --help             # 所有参数
```

## 2. GStreamer

`gst` feature 是把 `parlor` 从“会话读取器”变成“播放器”的东西。不带它时只有 `--render null`：
接收端接受会话、解密并统计媒体，但不播放。这在没有声卡的机器上测试网络或配对很有用，也是这个
feature 可选的原因。

### 本程序实际用到的插件

| 元素 | 软件包 | 用途 |
| --- | --- | --- |
| `h264parse`, `h265parse` | gst-plugins-base | 把投屏流拆成帧 |
| `avdec_h264`, `avdec_h265` | gst-libav | 软件解码（回退） |
| `d3d11h264dec`, `d3d11h265dec`, `d3d11videosink` | gst-plugins-bad（Windows） | 硬件解码与视频窗口 |
| `vah264dec`, `vah265dec` | gst-plugins-bad（Linux，VA-API） | 硬件解码 |
| `wasapisink` | gst-plugins-base（Windows） | 音频 |
| `pulsesink`, `alsasink` | gst-plugins-good/base（Linux） | 音频 |
| `autovideosink`, `glimagesink` | gst-plugins-base | 没有硬件 sink 时的视频窗口 |

### Windows

从 <https://gstreamer.freedesktop.org/download/> 安装 GStreamer 1.0 的 **MSVC 64-bit** 运行时与
开发包安装程序（选 “Complete” 安装），然后让运行时能被找到：

```powershell
# 执行后需要新开一个终端
setx PATH "%PATH%;C:\gstreamer\1.0\msvc_x86_64\bin"
```

如果你装在别处，改为把 `GST_PLUGIN_PATH_1_0` 指向它的 `lib\gstreamer-1.0` 目录，而不是加进
`PATH`。

发布压缩包是用 GNU 工具链针对 **MSYS2 UCRT64** 的 GStreamer 构建的。如果你想完全对齐：

```sh
pacman -S mingw-w64-ucrt-x86_64-gstreamer mingw-w64-ucrt-x86_64-gst-plugins-base \
          mingw-w64-ucrt-x86_64-gst-plugins-bad mingw-w64-ucrt-x86_64-gst-libav
```

两种运行时都行；DLL 是运行时找的，不是链接进去的。

### Linux

```sh
# Debian、Ubuntu
sudo apt install gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
                 gstreamer1.0-plugins-bad gstreamer1.0-libav gstreamer1.0-tools

# Fedora
sudo dnf install gstreamer1-plugins-base gstreamer1-plugins-good \
                 gstreamer1-plugins-bad-free gstreamer1-libav gstreamer1-plugins-bad-freeworld
```

构建还需要开发文件：

```sh
sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev
```

Linux 上的硬件解码走 VA-API：安装 `gstreamer1.0-vaapi`，并确认内核驱动已加载（常用检查是
`vainfo`）。

### 检查安装

```sh
gst-inspect-1.0 --version
gst-inspect-1.0 h264parse        # 必须存在
gst-inspect-1.0 d3d11h264dec     # 仅 Windows
gst-inspect-1.0 vah264dec        # 仅 Linux
gst-launch-1.0 --version
```

然后直接问 `parlor`：`./parlor --render gst -v` 会打印它要构建的管线，或者说明为什么建不了并
退出。

### 环境变量

| 变量 | 作用 |
| --- | --- |
| `AIRPLAY_GST_LAUNCH` | `gst-launch-1.0` 不在 `PATH` 上时，视频管线所用的程序 |
| `AIRPLAY_GST_AUDIO_SINK` | 未给 `-as` 时的默认音频 sink |
| `AIRPLAY_GST_VIDEO_SINK` | 未给 `-vs` 时的默认视频 sink |
| `AIRPLAY_GST_VIDEO_WARM` | 为 `1` 时在会话之间保留视频管线 |
| `AIRPLAY_GST_KEEP_STDERR` | 为 `1` 时保留管线自己的输出，用于诊断画面 |
| `AIRPLAY_DUMP_VIDEO` | 把最初的视频帧以 Annex-B 写入该路径 |
| `AIRPLAY_DUMP_AUDIO` | 把音频负载写入该路径 |

## 3. 首次运行与配对

```sh
./parlor -n "Living room" -pin 1234
```

应该看到：

```text
parlor 0.3.2 - an AirPlay receiver

  Name      Living room
  Port      7000
  Playback  GStreamer
  Clients   0 remembered

Waiting for a client. Press Ctrl+C to stop.
```

在 iPhone 或 iPad 上：打开控制中心，点**屏幕镜像**，选中接收端，被要求时输入 PIN。`-pin 1234`
的 PIN 是固定的；不带值（`-pin`）时接收端会生成一次性 PIN，并在客户端请求时打印：

```text
*** enter PIN 4821 on the client ***
```

口令同理，它守的是 `SETUP`：`-pw` 生成并打印一个，`-pw hunter2` 固定为它。口令至少四个字符。

配对过一次的客户端会记在 `clients` 文件里，不用再输 PIN。`identity.seed` 里的身份让接收端在重启
后仍被认出；删掉它，每台设备都要重新配对。

## 4. 日常使用

- **投屏**在设备端开始时启动；视频窗口此时出现，投屏停止时关闭。
- **音频**跟随会话；`-vol` 设置会话起始音量，`-db` 与 `-taper` 改变滑块到音量的映射。
- **停止**：Ctrl+C。退出时除了日志里的一段会话摘要，不写任何东西。
- **多会话**：第二个客户端同样会被服务；`-nohold` 则让新客户端顶替当前连接。
- **选择管线**：`-vd` 指定 H.264 解码器，`--video-decoder-hevc` 指定 H.265 解码器，`-vs` 指定
  sink。自 0.3 起 sink 不再自带解码器，所以硬件 sink 配默认解码器时是在 CPU 上解码。Windows
  上通常是 `-vs d3d11videosink -vd d3d11h264dec`。
- **平滑突发流**：`--pace-video` 让每一帧按其采集间隔输出，每帧最多 40 ms。它默认关闭；均匀
  到达的流不需要它。

## 5. 参数

先短名，同一行给出长名。每个值都在读取处校验；超出范围是错误，不是截断。

### 身份与访问

| 参数 | 含义 |
| --- | --- |
| `-n <name>`, `--name <name>` | 客户端显示的名字；默认 `parlor` |
| `-nh` | 为兼容而接受；本接收端从不在名字后附加主机名 |
| `-pin [nnnn]`, `--pin [nnnn]` | 要求 PIN；不给值时是一次性 PIN |
| `--no-pin` | 不要求 PIN 即可配对 |
| `-pw [password]`, `--password <password>` | 要求口令；`-pw` 不带口令时生成并打印 |
| `--random-password` | 为本次运行生成口令并打印 |
| `--password-file <path>` | 从文件读口令，路径为 `-` 时读标准输入 |
| `-key [file]`, `--key-file <file>` | 身份种子的存放位置；不给文件则什么都不保存 |
| `--seed <hex>` | 固定种子：64 个十六进制字符 |
| `--seed-file <path>` | 从该文件读取种子 |
| `--ephemeral-seed` | 只在本次运行使用的随机种子 |
| `-reg [file]`, `--clients-file <file>` | 已记住客户端的存放位置 |
| `--data-dir <path>` | 身份与客户端列表所在的目录 |
| `-m [mac]`, `--device-id <mac>` | 要广播的硬件地址 |
| `-restrict`, `--restrict` | 只允许 `-allow` 指定的设备 |
| `-allow <id>`, `--allow <id>` | 允许连接的设备；可重复 |
| `-block <id>`, `--block <id>` | 永不接受的设备；可重复 |

设备标识会归一化成 12 位十六进制，所以 `aa:bb:cc:dd:ee:ff`、`11-22-33-44-55-66` 与
`aabbccddeeff` 是同一台设备；其他值一律拒绝。只写 `-allow` 会自动开启 `-restrict` 并说明
这一点。这些列表比对的是客户端*自称*的身份，所以只是便利手段，不是安全边界：闸门是 PIN 与
口令。

### 网络与端口

| 参数 | 含义 |
| --- | --- |
| `--port <n>` | 控制端口；默认 7000，`0` 表示让系统选择 |
| `--bind <addr>` | 监听的地址；默认全部网卡 |
| `--no-advertise` | 不通过 mDNS 广播 |
| `-p` | 协议固定的媒体端口布局 |
| `-p <n>`, `--base-port <n>` | 从 `n` 开始的该布局：timing `n`、control `n+1`、data `n+2`、mirror `n`（TCP） |
| `--legacy-ports` | 固定布局：UDP 7011 timing、6001 control、6000 data、TCP 7100 mirror |
| `--timing-port <n>`, `--udp-control-port <n>`, `--data-port <n>`, `--mirror-port <n>` | 单独移动一个媒体端口 |
| `--nohold` | 新客户端接管当前客户端 |
| `--reset <seconds>` | 客户端停止发送 `/feedback` 这么久后关闭会话；0 表示从不关闭（默认），最大 86400 |

没有基准端口时，媒体端口由系统选择，并在 `SETUP` 里告诉客户端，因此不需要做端口转发。
`--port 0` 加 `--no-advertise` 得到的接收端，只有被告知地址的客户端才能连上。

### 显示与视频

| 参数 | 含义 |
| --- | --- |
| `--render <null|gst>` | 谁播放媒体：`null` 读取但不播放，`gst` 交给 GStreamer（带该 feature 的构建默认如此） |
| `-s <w>x<h>[@<r>]` | 提供给客户端的显示；默认 1920x1080@60，每边最大 16384 像素，刷新率 1 到 240 |
| `--width <px>`, `--height <px>`, `--refresh-rate <hz>` | 分开指定同一个显示 |
| `-fps <fps>`, `--max-fps <fps>` | 要求客户端的最高帧率；默认 30，取 1 到 240 |
| `-o`, `--overscanned` | 提供 overscan 显示 |
| `-h265`, `--hevc` | 允许 HEVC；未给尺寸时提供 4K |
| `--features <hex>` | 用这个特性位图代替默认值广播 |
| `-as <sink>`, `--audio-sink <sink>` | GStreamer 音频 sink |
| `-vs <sink>`, `--video-sink <sink>` | GStreamer 视频 sink |
| `-vp <parser>`, `--video-parser <parser>` | 镜像流的 parser |
| `-vd <decoder>`, `--video-decoder <decoder>` | H.264 流的解码器 |
| `--video-decoder-hevc <decoder>` | H.265 流的解码器 |
| `-avdec` | 两种编码都强制软件解码器 |
| `--warm-video` | 在会话之间保留视频管线 |
| `--pace-video`, `--no-pace-video` | 让帧按采集间隔输出（默认关闭） |

sink 与解码器是两个选择：只给 `-vs d3d11videosink` 会保留软件解码器。没有 `-vd` 的运行会在
启动时说明这一点。

### 音频

| 参数 | 含义 |
| --- | --- |
| `-al <seconds>`, `--audio-latency <seconds>` | 客户端为其音频排程的延迟；默认 0.25 秒，取 0 到 10 |
| `-vol <0..1>`, `--volume <0..1>` | 会话起始音量 |
| `-db <low:high>`, `--db <low:high>` | 把音量滑块重映射到分贝范围；默认 `-30:0` |
| `-taper`, `--taper` | 使用锥形音量曲线 |
| `--audio-codec <alac\|aac-eld>` | 客户端没说时会话回退的编码 |

`-al` 是刻意的延迟，不是缺陷：客户端把音频放在画面之后这么多，以保持两者同步。想要更跟手就
调低（`-al 0.05`），音频断断续续就调高。

### 客户端可写入的文件

| 参数 | 含义 |
| --- | --- |
| `-ca [file]`, `--cover-art [file]` | 把封面图写入该文件；不给文件时报告其类型与大小 |
| `-md <file>`, `--metadata <file>` | 把当前播放清单写入该文件 |
| `-dacp [file]`, `--dacp [file]` | 导出遥控标识；不给文件时用默认路径 |

在把这些路径指向重要的东西之前，先看[安全](#10-安全)。

### 日志与诊断

| 参数 | 含义 |
| --- | --- |
| `-q`, `--quiet` | 只有错误 |
| 默认 | 警告与错误 |
| `-v`, `--verbose` | 再加上会话记录 |
| `-d` | 再加上协议细节 |
| `--log-level <l>` | `off`、`error`、`warn`、`info`、`debug`、`trace` |
| `--log <file>` | 把请求轨迹追加到该文件 |
| `-FPSdata`, `--fps-data` | 记录客户端自己的流报告 |
| `-V`, `--version` | 打印版本 |
| `-h`, `-help`, `--help` | 打印用法文本 |

日志走标准错误。标准输出只保留标题、PIN 与生成的口令，所以脚本可以在没有自己的显示的情况下
抓取诊断信息。

### 取值与冲突

- 命令行若给出两件互相矛盾的事，就以退出码 2 结束，而不是替你选：身份只能来自 `--seed`、
  `--seed-file`、`-key <file>` 与 `-key` 之一；`-h265` 与 `--features` 不能同时设置特性位；
  口令只能由 `-pw`、`--password`、`--random-password` 与 `--password-file` 之一设置；
  `-p <base>` 不能与 `--port` 或单个媒体端口同给。
- 带值参数不会把下一个参数当成自己的值：`-n -p 7000` 会报 `-n` 需要值，而不是把接收端命名为
  `-p`。数值参数仍接受开头的负号，所以 `-db -30:0` 是一个范围。
- 浮点数会检查有限性：NaN 与无穷大一律拒绝，不做截断。

## 6. 常用配方

```sh
# 客厅里的接收端，固定 PIN
parlor -n "Living room" -pin 1234

# 没有声卡时检查网络与配对
parlor --render null -v

# 1080p30 下用硬件解码器取最低延迟
parlor -s 1920x1080 -fps 30 -vs d3d11videosink -vd d3d11h264dec

# 4K HEVC，两个解码器都显式指定
parlor -h265 -s 3840x2160 -vs d3d11videosink -vd d3d11h264dec --video-decoder-hevc d3d11h265dec

# 什么都不留下：没有身份文件，没有客户端列表
parlor -key --ephemeral-seed

# 只本机：与被告知 127.0.0.1:7000 的客户端配对
parlor --bind 127.0.0.1 --no-advertise

# 只允许一台设备
parlor -allow aa:bb:cc:dd:ee:ff

# 保留封面图与当前播放信息
parlor -ca cover.png -md now-playing.txt

# 不会出现在进程列表里的口令
parlor --password-file /run/secrets/parlor
```

## 7. 文件

| 文件 | 默认位置 | 说明 |
| --- | --- | --- |
| `identity.seed` | `%LOCALAPPDATA%\parlor`、`$XDG_DATA_HOME/parlor` 或 `$HOME/.local/share/parlor` | 64 个十六进制字符；所有密钥都由它派生 |
| `clients` | 与它同目录 | 每行一个 64 字符十六进制密钥：配对过的客户端 |

`--data-dir <path>` 会把两个文件一起搬走。`-key` 与 `-reg` 各搬一个。Unix 上种子以 0600 创建，
只有所有者可读。已存在却读不出来的种子会停止运行：它被报告而不是被替换，因为换新种子等于丢掉
所有配对。

`-key` 不带文件时，身份只在本次运行的内存里；这时客户端列表也留在内存里，除非 `-reg` 指定了
文件。

## 8. 退出码

| 退出码 | 含义 |
| --- | --- |
| 0 | 运行成功；`--help` 与 `--version` 也是 |
| 1 | 运行无法启动或失败：读不到身份、读不到口令文件、缺少后端、端口已被占用 |
| 2 | 命令行无法接受：未知参数、缺值、取值超出范围、两个参数互相矛盾 |

## 9. 故障排查

先从 `-v` 开始；它会打印正在使用的管线，以及接收端对自己这条流的判断。

### 完全没有画面

1. `-v` 应显示 `*** mirror stream keyed`，然后是 `video stream open in ... ms`。如果第一行没有，
   设备根本没到镜像端口：检查防火墙（控制端口 7000 与镜像 TCP 端口），以及两者是否在同一网络。
2. 如果显示 `rust-airplay: cannot show video: ...`，说明管线没建起来。先看原因；通常是缺元素
   （`gst-inspect-1.0 d3d11h264dec`），或者某个 sink 不能接在那个解码器后面。用 `-vd avdec_h264`
   回退。
3. 窗口由 sink 在投屏开始时创建，不会抢焦点。到任务栏或 Alt+Tab 里找它。
4. 没有 `gst` feature 的构建会以退出码 1 拒绝 `--render gst`，并说明原因。

### 画面停了，设备还在发

这是 0.3.1 接收端的缺陷：队列溢出一次之后，它会等待一个静止画面永远不会发送的关键帧。0.3.2
修复了它，本发布用的就是 0.3.2。如果在 0.3.2 上看到 `written` 不再增长而 `received` 仍在增长，
请附日志报告。

### 画面卡顿

```text
media     mirror: 308 packets, 3946087 bytes in, 297 received, 297 written, 1 waiting, 0 dropped
```

- `waiting` 到 8 且 `dropped` 持续增长：渲染端跟不上。改用硬件解码器（`-vd d3d11h264dec`）、
  要求更小的画面（`-s 1280x720`），或降低 `-fps`。
- `pace frame` 行里的 `queued` 达到数百毫秒，说的是同一件事。
- 以突发方式到达的流即使什么都没丢也会抖；`--pace-video` 把帧摊开，代价是每帧最多 40 ms。

### 没有声音

`-v` 应显示 `audio stream open, aac-eld at 44100`。如果没有，客户端没有开始音频。如果管线打不开
sink，显式指定一个：Windows 上用 `-as wasapisink`，Linux 上用 `-as pulsesink` 或 `-as alsasink`，
并用 `gst-inspect-1.0` 检查它。

### PIN 一直不出现

固定 PIN（`-pin 1234`）不会打印；一次性 PIN 在客户端请求时打印。用 `--no-pin` 时根本不用 PIN。

### 端口已被占用

运行会以退出码 1 结束，并指出端口。另一个接收端或别的程序占用着它：用 `--port` 换一个，或者用
`--port 0` 让系统选（然后靠发现机制找到接收端，而不是手动输入端口）。

### `--render gst` 说缺少该 feature

二进制构建时没带 `--features gst`。带上它重新构建，或者用 `--render null`。

### 找不到 gst-launch-1.0

视频管线运行在 `gst-launch-1.0` 里。把 GStreamer 的 `bin` 放进 `PATH`，或者用
`AIRPLAY_GST_LAUNCH` 指定程序。音频不受影响；视频不行。

## 10. 安全

- **PIN 与口令才是闸门。** `-allow` 与 `-block` 比对的是客户端自称的设备标识，恶意客户端
  可以随便自称。
- **`-ca`、`-md` 与 `-dacp` 接受来自网络的、未认证的写入。** 局域网内任何主机都能发请求，让程序
  写文件、打印当前播放文本。程序限制了这种请求能做的事——去掉控制字符、终端输出截断到 2 KiB、
  封面图超过 4 MiB 与元数据超过 256 KiB 则丢弃、写入走有界队列——但请把这些路径指向无关紧要的
  位置，并把文件当作公开文件。
- **不要把凭据写在命令行上。** `--password` 与 `--seed` 在进程列表与 shell 历史里可见；改用
  `--password-file -`（标准输入）与 `--seed-file`。
- **种子是接收端的身份。** 拿到 `identity.seed` 的人可以冒充这个接收端。Unix 上它以 0600 创建；
  Windows 上它依赖用户目录的 ACL。
- 协议本身是带 MD5 摘要的未加密 RTSP，它是为家庭网络设计的。
