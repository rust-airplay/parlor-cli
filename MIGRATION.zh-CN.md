# 迁移

English: [MIGRATION.md](MIGRATION.md)。

这里描述四次迁移：程序从库的工作区里独立成仓库（这一步成为 0.2.0）、0.3.0 的命令行变化、0.3.1 的库修复，以及 0.3.2 的 git 依赖。

## 0.3.1 → 0.3.2

配置上不需要改任何东西，但构建方式有一点要知道。

1. **两个库改为 git 依赖。** `Cargo.toml` 里写的是各自的提交号而不是版本号，因为修好"画面冻结"的版本走在 crates.io 之前。首次构建会克隆它们，之后 `--offline` 可用；0.3.1 的锁文件需要重新生成（`cargo update`）。
2. **冻结问题已修。** rust-airplay 0.3.2 给"溢出后等待关键帧"加上了超时，客户端没有新画面时也不会再让会话停住。命令行没有变化。
3. **其余与 0.3.1 相同。**

## 0.3.0 → 0.3.1

配置上不需要改任何东西；这次发布改的是底下的库。

1. **把依赖移到 0.3.1。** `rust-airplay` 与 `rust-airplay-gstream` 0.3.0 带着一处节拍缺陷：接收端把每帧的到达间隔报告给后端，后端在写之前睡掉这个间隔，于是渲染器永远落后于流，帧队列也就永远排不空。0.3.0 上的投屏会卡，而且没有任何参数能修好。
2. **如果命令行里的 `--pace-video` 只是当初的绕过手段，可以去掉。** 它现在默认关闭，这本来就是从设备到屏幕最短的路径。若流是突发到达的，它仍然有用，代价是每帧最多 40 ms。
3. **其余不变。** 参数集合、退出码、文件及其格式都与 0.3.0 相同。

## 0.2.0 → 0.3.0

接收端本身没变：默认数据目录、参数名、两个文件的格式都没变。变的是命令行的边界情况、退出码，以及代码的组织方式。

### 需要做的事

1. **把依赖移到 0.3。** 两个库都已发布到 crates.io，没有要先更新的 Git 或路径依赖，也没有要跟的分支。锁在 0.2 上的 lock 文件需要重新生成。
2. **让脚本认识退出码。** 无法接受的命令行现在退出 2，而不是 1。把所有非零都当成“接收端启动失败”的脚本，需要区分 2（改命令行）与 1（看接收端）。0 仍是成功，`--help` 与 `--version` 也是成功。
3. **检查现在会冲突的命令行。** 以下情况过去会被静默取其一，现在以退出码 2 拒绝：
   - `--seed`、`--seed-file`、`-key <file>`、`-key` 中出现两个身份来源；
   - `-h265` 与 `--features` 同给；
   - `-pw`、`--password`、`--random-password` 与 `--password-file` 同给；
   - `-p <base>` 与 `--port` 或 `--timing-port`、`--udp-control-port`、`--data-port`、`--mirror-port` 同给。
4. **检查只写 `-allow` 的用法。** 现在只给 `-allow` 会自动开启 `-restrict` 并打印警告。过去带 allow 列表又没有 `-restrict` 的接收端会接受所有人，现在只接受列表里的设备。如果本意是保持旧行为，改用 `-block` 列出要拒绝的设备，或者干脆去掉这个列表。
5. **检查传入的标识。** `-allow` 与 `-block` 收 12 位十六进制，并归一化分隔符：`aa:bb:cc:dd:ee:ff`、`11-22-33-44-55-66` 与 `aabbccddeeff` 是同一台设备。不是 12 位十六进制的值现在会报错，过去会被接受、然后匹配不到任何设备。
6. **读标准输出找警告的地方改到标准错误。** 所有日志都走标准错误；标准输出只剩标题、PIN 和生成的口令。
7. **检查生成的参数里的 NaN 与无穷大。** `-vol`、`--volume`、`-al`、`--audio-latency` 与 `-reset` 拒绝它们；`-s` 每边超过 16384、刷新率不在 1 到 240、以及 `-reset` 超过 86400 秒也一样拒绝。过去能通过的生成命令行现在会退出 2。
8. **检查 `-h265` 与显式尺寸的组合。** 只有没给尺寸时 `-h265` 才把默认显示放大到 4K。`-h265 -s 1920x1080` 保持 1080p；依赖过无条件放大行为的命令需要自己写上 4K 尺寸。
9. **读 `-ca` 输出的地方改到标准错误。** `-ca` 不带路径现在把每张图的内容类型与字节数报告到标准错误，而不是什么都不做。它不写磁盘，只盯着文件产出的管线不受影响。

### 身份文件损坏

0.3.0 只在 `identity.seed` 不存在时新建；文件存在却读不出来时，它以退出码 1 停止。它不会悄悄换一个新身份，因为那会丢掉旧身份的所有配对。

如果程序以 `parlor: cannot read <path>: ...` 加退出码 1 停止，按顺序处理：

1. 确认路径存在且是文件。如果是由 `--data-dir`、`-key` 或 `--key-file` 指定的，消息里就是实际用的路径。
2. 确认运行 `parlor` 的账号能读它。Unix 上 `ls -l` 应显示该用户为所有者、权限 0600；从别的账号拷来的种子可能属于那个账号。
3. 检查内容：文件是 32 字节，写成 64 个十六进制字符。被文本编辑器存成 UTF-16，或手改后剩下 63 / 65 个字符，是最常见的损坏。
4. 有备份就恢复。这是唯一保住旧配对的办法。
5. 如果接受重新配对，把坏文件挪开再启动。程序会新建身份；所有客户端都要重新配对。`clients` 文件可以留着，但里面的密钥属于旧身份。

Windows：

```powershell
Move-Item "$env:LOCALAPPDATA\parlor\identity.seed" "$env:LOCALAPPDATA\parlor\identity.seed.broken"
```

Unix：

```sh
mv "${XDG_DATA_HOME:-$HOME/.local/share}/parlor/identity.seed" \
   "${XDG_DATA_HOME:-$HOME/.local/share}/parlor/identity.seed.broken"
```

不要直接删掉文件，也不要把 `--data-dir` 指向空目录，除非你确实想丢掉配对。

### 依赖

```toml
rust-airplay = "0.3"
rust-airplay-gstream = { version = "0.3", optional = true }
```

两者都已发布到 crates.io。没有 Git 依赖，也没有路径依赖：只要有 crates.io（或镜像、或 vendor 目录）能提供这两个版本，这个仓库不需要旁边放着两个库的仓库就能构建。`gst` feature 默认关闭，`--render null` 既不需要 GStreamer，也不需要后端。

### 代码结构

`src/main.rs` 不再承载程序。它是入口，过去做的一切都在 `parlor` 库里：

| 之前 | 现在 |
| --- | --- |
| 单个 `src/main.rs` | `src/lib.rs` 加上 `cli`、`plan`、`files`、`console`、`render` |
| 二进制里的私有函数 | 名为 `parlor` 的库的 `pub` 模块 |
| 没有库目标 | `[lib] name = "parlor"`，与 `parlor` 二进制并列 |

只运行二进制的人不需要改动：二进制仍叫 `parlor`。

## 程序独立成仓库（0.1 → 0.2.0）

这个 crate 原本是库工作区里的 `crates/rust-airplay-cli`，产出的可执行文件叫 `rust-airplay`。现在它是 `parlor-cli` 仓库，产出 `parlor`。

### 变化

| 之前 | 现在 |
| --- | --- |
| 包名 `rust-airplay-cli` | 包名 `parlor-cli` |
| 可执行文件 `rust-airplay` | 可执行文件 `parlor` |
| 默认接收端名 `rust-airplay` | 默认接收端名 `parlor` |
| 数据目录 `rust-airplay/` | 数据目录 `parlor/` |
| 路径依赖 | crates.io 上已发布的版本 |
| `rust_airplay_gstreamer::` | `rust_airplay_gstream::` |

拆分时两个库还是 `main` 上的 Git 依赖；0.2.0 与 0.3.0 用的是已发布的版本，Git 阶段已经结束。

数据目录改了名字，因此已有的配对不会自动沿用。要保留身份，把目录移过去即可：

```powershell
Move-Item "$env:LOCALAPPDATA\rust-airplay" "$env:LOCALAPPDATA\parlor"
```

## 门禁

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features gst -- -D warnings
cargo test
cargo test --features gst
cargo build --release --features gst
```

这里现在有测试，而且不需要 GStreamer。`src/` 有 33 个单元测试：参数解析器、显示与端口计划、种子与文件写入、控制台的可打印文本。`tests/cli.rs` 有 7 个集成测试，通过 `CARGO_BIN_EXE_parlor` 运行构建出的二进制：退出码、`--help` 里的参数清单、`--version`、非十六进制的种子、被关闭的输出管道、本构建没有的后端，以及一个真正启动并保住身份的接收端。请求 GStreamer 后端的那个只适用于没有该 feature 的构建，所以 `cargo test --features gst` 跑 6 个。
