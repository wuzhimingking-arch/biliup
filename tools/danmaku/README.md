# biliup Bilibili 弹幕修复定制版

本分支基于官方 [biliup](https://github.com/biliup/biliup) v1.2.11，完整保留官方历史与 LICENSE，当前版本 `1.2.11-danmaku.4`。这是个人定制分支，不是官方发布。

## 改动与来源

原源码基线 `3d884a570be7761e65413785cfc63d88a97ef0e5`。从本地既有提交迁移已验收内容，不重新实现：

| 原本地提交 | 内容 | 本分支处理 |
|---|---|---|
| f4fccf5 | 认证校验、备用节点、心跳/重连诊断、XML 回归、本地隔离模式 | 保留程序改动和全部回归；现场工具替换成通用入口 |
| 2ff2c45 | 配置页防止浏览器自动填充导致保存失败 | 保留 6 项 autocomplete 设置 |
| f471213 | 早期真实实录记录 | 仅保留脱敏历史结论 |
| 653fe11 | 最后 NAS 根因修复：Cookie 与 UID 一致性 | 完整保留协议代码、探针及身份校验测试 |
| a546481 | NAS 部署与压制降级 | 仅整理 2 个通用对接脚本及隔离回归；不发布生产部署脚本/数据 |

最后 NAS 修复确实涉及源码：已登录 Cookie 获取的弹幕 token 不能配合匿名 UID=0 认证。共享协议入口现在使用 Cookie 中 DedeUserID 推导缺失 UID，并拒绝无效、重复冲突或与显式 UID 不一致的身份；不记录真实 UID、Cookie 或 token。

还保留非空 token 校验、有效节点去重、协议认证响应 code=0、10 秒超时、认证后心跳、多节点恢复，以及 WebSocket 帧/协议包/事件/XML 写入计数。XML 有刷新和幂等关闭回归，原视频分段与弹幕路径的对应逻辑保留。其他平台仍使用各自协议。

## 正式与测试模式

| 环境变量 | 行为 |
|---|---|
| 未设置或 `0` | 原有录制、配置、账号、投稿、分段钩子和投稿后处理保持正常 |
| 显式 `1` | 临时强制 B 站弹幕、Mesio 下载/Noop 投稿、保留短片、清除测试 worker 的投稿关联和所有录制钩子；不改写数据库 |

正式部署不要使用 1。根 Dockerfile 沿用官方行为；`local-test/Dockerfile` 是已编译二进制的最小运行时，默认变量为 0，保留 `server --bind 0.0.0.0 --auth`。没有内置账号、Cookie、NAS 地址或生产房间。

## 构建与验证

使用 Rust 1.99.1（本次验证工具链）和 Node 24，先 `npm ci && npm run build` 生成 out/，再运行：

```sh
bash tools/danmaku/check.sh
```

脚本只格式检查受影响 Rust 文件，不为整理格式重写其他模块；运行全部 danmaku 单元/文档测试、Worker 配置回归，在 0 和未设置两种情况下检查正式投稿关联/钩子不被清除，再检查探针编译和构建 release CLI。可复用现有 Cargo/npm 缓存；无需访问 NAS、登录 B 站或公开投稿。

通用 Docker 构建沿用根目录官方 Dockerfile：

```sh
docker build --platform linux/amd64 -t biliup-custom:danmaku .
```

如已有 linux/amd64 CLI，可使用最小运行时（只用于本分支 1.2.11，避免漏掉新版运行资源）：

```sh
mkdir -p local-test/.build
cp target/release/biliup local-test/.build/biliup
docker build --platform linux/amd64 --build-arg SOURCE_REVISION="$(git rev-parse HEAD)" --build-arg BINARY_SHA256="$(sha256sum local-test/.build/biliup | cut -d' ' -f1)" -f local-test/Dockerfile -t biliup-custom:danmaku-runtime local-test/.build
```

正式部署配置示例见 `examples/nas-danmaku-hooks/compose.yaml`，不包含现场数据。外部压制集成见该目录 README；没有把独立 biliup-burn worker 放入主仓库。

协议探针复用生产 recorder。登录 Cookie 文件留在设备本机，只传文件路径；HTTP 解压功能须与 CLI 一致：

```sh
cargo run --locked -p danmaku --example record_probe --features reqwest/gzip,reqwest/brotli -- https://live.bilibili.com/ROOM_ID /tmp/probe.xml 180 /path/to/account.json infer
```

不带账号参数则匿名；`cookie` 显式使用文件中的账号 UID，`infer` 用修复逻辑推导，`zero` 与登录 Cookie 冲突时会在连接前拒绝。不得把这种提前拒绝误报为服务器认证成功。

## 历史真实验收与本次范围

以下是 `.3` 的历史实机结果，本轮没有重新连接 NAS、录制直播、部署或投稿：

- 原 `.2` 早期匿名样本约 129 秒、6 条弹幕，不能覆盖最后登录 Cookie 的故障。
- 同一旧程序在 Windows 与 NAS 均复现 Cookie+UID0 重置，只有 UID 改为账号 UID 就认证成功；不是简单换网络或升级版本。
- `.3` NAS 正式配置自然分段 XML 闭合、779 条真实弹幕；独立 A 样本 196 秒/172 条，B Cookie Bridge 后续样本 646 秒/727 条/22 次心跳、无 I/O 错误，完整视频解码通过。
- `.3` Windows 正式模式 Host 样本 356 秒/729 条、14 次心跳，完整解码通过。本地 Bridge 曾停滞，保留该限制。
- 外部压制容器真实 XML→ASS→VAAPI 以及真实队列的独立短片测试通过。错误的持久钩子 `run = sh ...` 另行停写备份后纠正；这不是本程序的网络修复。

本轮执行结果记入 DEV_LOG.md；`.4` 没有再次通过 NAS 实机验收，不能把源码整理后的编译/单元测试称为重新实机部署成功。实际平台投稿末端及自然断线后的实网恢复未重新验证。

## 同步官方更新

origin 为个人 Fork，upstream 为官方。Fork master 保持官方 1.3.0；定制分支基于 1.2.11。当前直接三方应用会在客户端和 B 站协议冲突，不能用整个旧文件替换新版。

```sh
git fetch upstream --tags
git switch -c review/danmaku-next peakedge/danmaku-fix
git merge upstream/master
# 在 review 分支逐项解决冲突，保留新版功能，再执行完整回归；通过后才合回定制分支。
```

冲突未解决时可 `git merge --abort` 回到 review 分支起点。只向个人 Fork 推送，不强推、不向官方提交 PR。此分支尚不承诺与 1.3.0 的新数据结构和内置压制流程兼容。
