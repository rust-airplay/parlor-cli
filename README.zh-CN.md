# parlor-cli

[![CI](https://github.com/rust-airplay/parlor-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/rust-airplay/parlor-cli/actions/workflows/ci.yml)

`parlor`：命令行上的 AirPlay 接收端。

它是程序，不是协议。接收端本身是 [`rust-airplay`](https://github.com/rust-airplay/rust-airplay)，播放由 [`rust-airplay-gstream`](https://github.com/rust-airplay/rust-airplay-gstream) 负责；这个 crate 解析参数、保存两个文件、安装 logger 并打印。它不含任何协议代码。

运行逻辑本身是一个名为 `parlor` 的库，别的程序不必经过二进制就能启动接收端；`src/main.rs` 只负责转交。组成部分是：`src/cli.rs` 的命令行、`src/plan.rs` 对命令行的解释、`src/files.rs` 的文件与写入线程、`src/console.rs` 的标题与日志、以及 `src/render.rs` 的后端。

## 构建

```sh
cargo build --release
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
parlor 0.3.0 - an AirPlay receiver

  Name      Living room
  Port      7000
  Playback  GStreamer
  Clients   0 remembered

Waiting for a client. Press Ctrl+C to stop.
```

在 iPhone 或 iPad 上打开 AirPlay 菜单，选中接收端，在提示出现时输入 PIN。客户端一连上就开始镜像；客户端开始播放时就有声音。

`--bind <addr>` 限定接收端监听的地址；默认监听全部网卡。`--no-advertise` 让接收端不出现在 mDNS 里，客户端看不到它，必须被告知地址。`--version` 打印版本后退出。

## 控制台

标题只打印一次；日志级别隐藏警告时（例如 `-q` 或 `--log-level off`）它也不打印。之后控制台只显示警告与错误；PIN 或生成的口令在需要时打印，与日志级别无关。

所有日志都写标准错误。标准输出只保留给人看的内容：标题、PIN、生成的口令。这样脚本可以单独抓取诊断信息而不丢标题，守护进程也能在同一路输出里看到警告与致命错误。

| 参数 | 显示内容 |
| --- | --- |
| `-q`、`--quiet` | 只有错误 |
| 默认 | 警告与错误 |
| `-v`、`--verbose` | 再加上会话记录：流打开、时钟、媒体统计 |
| `-d` | 再加上协议细节 |
| `--log-level <level>` | 直接指定：`off`、`error`、`warn`、`info`、`debug`、`trace` |

`-q` 连标题一起隐藏：它只打印错误，而标题不是错误。PIN 或生成的口令仍会打印在标准输出。

`--log <file>` 把请求轨迹追加到文件，与以上级别无关。

## 参数

短名是 AirPlay 接收端惯用的写法；有长名的参数，下表把两者都给出，供偏好单词的脚本使用。

| 参数 | 含义 |
| --- | --- |
| `-n <name>`、`--name <name>` | 客户端显示的名字，默认 `parlor` |
| `-nh` | 接受；本接收端从不在名字后附加主机名 |
| `-h265`、`--hevc` | 允许 HEVC；未显式指定尺寸时提供 4K |
| `-pin [nnnn]`、`--pin [nnnn]` | 要求 PIN，可指定固定值；这是默认行为 |
| `--no-pin` | 不要求 PIN |
| `-pw [password]`、`--password <password>` | 要求口令；`-pw` 不带口令时生成并打印 |
| `--random-password` | 为本次运行生成口令并打印 |
| `--password-file <path>` | 从文件读口令；路径为 `-` 时读标准输入 |
| `-m [mac]`、`--device-id <mac>` | 广播指定的硬件地址 |
| `-key [file]`、`--key-file <file>` | 身份种子存放位置；不给文件则完全不保存 |
| `--ephemeral-seed` | 身份只在本次运行内有效 |
| `--seed <hex>` | 固定的种子：64 个十六进制字符 |
| `--seed-file <path>` | 从该文件读取种子 |
| `-reg [file]`、`--clients-file <file>` | 已记住客户端的存放位置 |
| `--data-dir <path>` | 身份与客户端列表所在的目录 |
| `-p [n]`、`--base-port <n>` | 固定媒体端口布局，或基准端口 `n` |
| `--legacy-ports` | 协议固定的媒体端口布局 |
| `--port <n>` | 控制端口；`0` 表示让系统选择 |
| `--bind <addr>` | 监听的地址；默认全部网卡 |
| `--no-advertise` | 不通过 mDNS 广播 |
| `--timing-port <n>`、`--udp-control-port <n>`、`--data-port <n>`、`--mirror-port <n>` | 单独移动一个媒体端口 |
| `-al <seconds>`、`--audio-latency <seconds>` | 客户端排程所用的音频延迟，默认 0.25 秒 |
| `-vol <0..1>`、`--volume <0..1>` | 会话起始音量 |
| `-db <low:high>`、`--db <low:high>` | 把音量滑块重映射到该分贝范围，默认 `-30:0` |
| `-taper`、`--taper` | 使用锥形音量曲线 |
| `-s <w>x<h>[@<r>]` | 提供给客户端的显示，默认 1920x1080@60 |
| `--width <px>`、`--height <px>`、`--refresh-rate <hz>` | 分开指定同一个显示 |
| `-fps <fps>`、`--max-fps <fps>` | 要求客户端的最高帧率，默认 30 |
| `-o`、`--overscanned` | 提供 overscan 显示 |
| `-as <sink>`、`--audio-sink <sink>` | GStreamer 音频 sink |
| `-vs <sink>`、`--video-sink <sink>` | GStreamer 视频 sink |
| `-vp <parser>`、`--video-parser <parser>` | 镜像管线使用的 parser |
| `-vd <decoder>`、`--video-decoder <decoder>` | H.264 镜像流使用的解码器 |
| `--video-decoder-hevc <decoder>` | H.265 镜像流使用的解码器 |
| `-avdec` | 强制使用软件解码器 |
| `--warm-video` | 在第一个客户端之前启动视频管线 |
| `--pace-video` | 让镜像帧按采集间隔输出 |
| `--render <backend>` | 用什么播放：`null` 或 `gst` |
| `--audio-codec <coding>` | 会话回退的编码：`alac` 或 `aac-eld` |
| `-ca [file]`、`--cover-art [file]` | 把封面图的类型与大小报告到标准错误，或写入文件 |
| `-md <file>`、`--metadata <file>` | 把当前播放信息写入文件 |
| `-dacp [file]`、`--dacp <file>` | 导出遥控标识 |
| `-reset <seconds>`、`--reset <seconds>` | 无 `/feedback` 超过 n 秒关闭会话；`0` 表示从不 |
| `-nohold`、`--nohold` | 新客户端顶替当前连接 |
| `-restrict`、`--restrict` | 只允许 `-allow` 指定的设备 |
| `-allow <id>`、`--allow <id>` | 允许连接的设备；可重复 |
| `-block <id>`、`--block <id>` | 永不接受的设备；可重复 |
| `-FPSdata`、`--fps-data` | 记录客户端自己的流报告 |
| `-q`、`--quiet` | 只有错误 |
| `-v`、`--verbose` | 警告、错误与会话记录 |
| `-d` | 全部，包括协议细节 |
| `--log-level <level>` | `off`、`error`、`warn`、`info`、`debug` 或 `trace` |
| `--log <file>` | 把请求轨迹追加到文件 |
| `-V`、`--version` | 打印版本并退出 |
| `-h`、`-help`、`--help` | 打印用法并退出 |

### 端口布局与口令

- `-p n` 按 `tcp`/`udp` 占 `n..n+2` 布置端口，控制端口是 `n+1`。只写 `-p` 选择固定布局：镜像 `7100`、控制 `7000`、timing `7011`、UDP 控制 `6001`、数据 `6000`。`--port` 单独移动控制端口，`--timing-port`、`--udp-control-port`、`--data-port`、`--mirror-port` 各移动一个媒体端口。
- `-pw` 不带口令时，本程序为整个进程生成一次并打印。`--password-file` 从文件读，路径为 `-` 时读标准输入，并去掉行尾；口令至少四个字符。生成的口令打印在标准输出。

### 客户端列表

`-allow` 与 `-block` 接受设备标识，并归一化成 12 位大写十六进制，因此 `aa:bb:cc:dd:ee:ff`、`11-22-33-44-55-66` 与 `aabbccddeeff` 是同一台设备；不是 12 位十六进制的值会直接报错，而不是悄悄匹配不到任何客户端。只给 `-allow` 会自动开启 `-restrict` 并打印一条警告，因为没有它库会忽略这个列表；`-restrict` 没有任何 `-allow` 时拒绝所有设备，并说明这一点。

### 冲突与取值

互相矛盾的命令行以退出码 2 拒绝，而不是静默取其一：

- 身份只能由 `--seed`、`--seed-file`、`-key <file>`、`-key`/`--ephemeral-seed` 四者之一给出；
- `-h265` 与 `--features` 都在设置广播的特性位；
- 口令只能由 `-pw`、`--password`、`--random-password`、`--password-file` 之一设置；
- `-p <base>` 决定整套端口布局，因此不能与 `--port` 或 `--timing-port`、`--udp-control-port`、`--data-port`、`--mirror-port` 同给。

带值参数不会把下一个参数当成自己的值：`-n -p 7000`、`-allow -restrict`、`-md -q` 会报“需要值”，而不是吞掉后面的参数。数值参数仍接受以 `-` 开头的值，所以 `-db -30:0` 是一个范围。

每个数值都在读取处校验：`-vol`、`--volume`、`-al`、`--audio-latency` 与 `-reset` 拒绝 NaN 与无穷大；`-s` 每边最多 16384 像素、刷新率 1 到 240；`-fps` 与 `--max-fps` 取 1 到 240；`-reset` 最多 86400 秒。

## 退出码

| 退出码 | 含义 |
| --- | --- |
| `0` | 运行成功，`--help` 与 `--version` 也是 |
| `1` | 运行无法启动或失败：读不到身份、读不到口令文件、本构建或本机没有所需后端 |
| `2` | 命令行无法接受：未知参数、参数缺值、取值超出范围，或两个参数互相矛盾 |

命令行无法接受时，原因写在标准错误，退出码 2；未知或不支持的参数还会附上用法文本。运行失败只打印一行说明哪里出错，退出码 1。

## 文件

程序在本地应用数据目录下保存两个文件：`parlor/identity.seed` 与 `parlor/clients`。Windows 上是 `%LOCALAPPDATA%\parlor`，Unix 上是 `$XDG_DATA_HOME/parlor` 或 `$HOME/.local/share/parlor`；`--data-dir <path>` 可以直接指定，如果这些变量一个都没有设置，就必须用它。

- `identity.seed` 首次运行时生成，是接收端跨重启保持可识别的原因。删除它会让所有客户端重新配对。Unix 上它以 0600 创建，只有所有者可读。它只在文件不存在时新建：文件存在却读不出来会报错并停止运行，因为换一个新身份等于悄悄丢掉所有配对。
- `clients` 每行一个 64 字符十六进制客户端密钥。删除它保留身份、只忘记客户端。只有新客户端才会追加，重连不会让文件变大。

`-key` 与 `-reg` 可以搬动这两个文件；`-key` 不带文件表示身份只存在于内存里，什么都不保存，客户端注册表也只留在内存，除非 `-reg` 指定了文件。

客户端能触发的写盘——封面图、当前播放信息、遥控标识——都交给同一个写入线程，队列长度 16。队列满时丢弃这次写入并告警，而不是拖住会话。

## 延迟

投屏看的是画面跟手的快慢。最快的路径是硬件路径，而它需要两个参数而不是一个：0.3 起 sink 与解码器是两个独立选择，`-vs d3d11videosink` 不再顺带换掉解码器。只指定硬件 sink、不指定解码器时，每帧都由 CPU 解（`avdec_h264`）；1080p 流在这种配置下只是勉强跟上，接收端那 8 帧的队列会一直是满的，一帧要等约四分之一秒才轮到渲染器，稍有抖动就丢帧。

```sh
parlor -vs d3d11videosink -vd d3d11h264dec
```

H.265 流（`-h265`）要单独指定解码器，因为一个硬件元素只解一种编码：

```sh
parlor -h265 -vs d3d11videosink -vd d3d11h264dec --video-decoder-hevc d3d11h265dec
```

没有给 `-vd` 时启动会打印一条警告；`-v` 会显示每一帧等在哪里：

```text
media     pace frame 12: queued 262 ms, pacer wanted 31 ms, waited 0 ms
media     400 video frames queued, 8 waiting
```

`queued` 达到几百毫秒、并且 `8 waiting`（队列上限）——这就是渲染器跟不上；只有几毫秒且 `0 waiting`，说明是接收端在等客户端。

## 已知限制

用 `-ca`、`-md` 或 `-dacp` 启动时，程序会接受客户端要求的写盘与终端输出，而协议不认证这个请求：局域网内任何主机都可以发一个 `SET_PARAMETER`，让程序写文件或打印一行。程序限制了这种请求能做的事——打印前去掉控制字符、终端文本截断到 2 KiB、封面图超过 4 MiB 与元数据超过 256 KiB 直接丢弃、写盘走有界队列——但真正的修复必须在库里做认证。在那之前，把这些参数指定的文件当作公开文件，指向一个不值得保护的位置。

## 测试

```sh
cargo test
cargo test --features gst
cargo fmt --all -- --check
cargo clippy --all-targets --features gst -- -D warnings
```

`src/` 里有 33 个单元测试：解析器、显示与端口计划、种子与文件写入、以及控制台的可打印文本。`tests/cli.rs` 有 7 个集成测试，通过 `CARGO_BIN_EXE_parlor` 运行构建出的二进制：退出码、`--help` 里的参数清单、`--version`、非十六进制的种子、被关闭的输出管道、本构建没有的后端，以及一个真正启动并保住身份的接收端。其中请求 GStreamer 后端的那个只在没有该 feature 的构建里运行，所以 `cargo test --features gst` 会跑 6 个。所有测试都不需要 GStreamer。

## 发布

`Release` 工作流手动触发，有三个输入：形如 `v0.3.0` 的 tag（缺少 `v` 会自动补上）、发布说明（留空则使用 `RELEASE_NOTES.md`，否则换行用 `\n`）、以及是否创建草稿。它使用 GStreamer 后端为 Linux 与 Windows 构建 `parlor`，把二进制、README 与许可证打包，并创建 GitHub release。Windows 构建使用与 MSYS2 GStreamer 库匹配的 GNU 工具链；两个压缩包都要求运行机器安装 GStreamer 1.0 及基础插件。

## 文档

- [rust-airplay 的 API](https://docs.rs/rust-airplay)：`Config`、渲染器 trait、各模块
- [rust-airplay 的后端指南](https://github.com/rust-airplay/rust-airplay/blob/main/docs/BACKEND.md)：如何实现播放后端
- [rust-airplay-gstream 的 README](https://github.com/rust-airplay/rust-airplay-gstream)：播放路径的 sink、元素与行为
- [CHANGELOG.md](CHANGELOG.md)：每个版本包含什么
- [MIGRATION.zh-CN.md](MIGRATION.zh-CN.md)：版本之间变了什么，需要做什么

## 许可证

GPL-3.0-or-later。见 [LICENSE](LICENSE)。

English: [README.md](README.md)。
