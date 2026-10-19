# 当前定制版本

- 官方基线：biliup v1.2.11，`3d884a570be7761e65413785cfc63d88a97ef0e5`。
- 定制版本：`1.2.11-danmaku.4`，分支 `peakedge/danmaku-fix`。
- Fork：https://github.com/wuzhimingking-arch/biliup；upstream：https://github.com/biliup/biliup.git。
- 保留最后 NAS 修复 `.3` 的全部程序改动：登录 Cookie 与弹幕 UID 一致性、非空 token/有效备用节点、认证 code/超时、认证后心跳、握手后失败切节点、退避重连、计数诊断和 XML 刷新/收尾。
- 保留 WebUI 配置页防止浏览器把本站登录凭据自动填入其他平台或数字字段的修复，不放松配置类型校验。
- 测试模式显式设为 1 才启用；普通进程默认关闭，提供的预编译运行时 Dockerfile 显式为 0。生产投稿模板、录制钩子及归档路径保持原逻辑。
- 压制对接仅提供独立通用示例，不包含外部 worker/压制镜像、现场路径或生产数据。以前的错误钩子配置 `run = sh ...` 属于部署错误，正确示例为 `{"run":"sh /opt/burn-hook-safe.sh"}`，不伪装为主程序修复。
- 官方 master 在本次检查为 1.3.0、`f6c8f211ba5bd8f0a9c05d8d500c6a7856dac86b`，相对基线有 13 个提交、472 个文件变化。三方试合并在 client.rs 与 bilibili.rs 冲突，未迁移到 1.3.0，也不合并或覆盖 Fork master。
- 原本地提交及现场记录留在原工作区；公开分支仅迁移代码、测试和通用文档。历史实录结果及本次验证范围见 tools/danmaku/README.md 与 DEV_LOG.md。
- 本轮受影响 Rust 格式、61 单元+1文档、3项 context 回归（另在未设置变量时复测正式模式）、探针编译、release CLI、WebUI/TypeScript、压制降级隔离回归及 Compose 校验通过；敏感信息检查通过。未连接 NAS 或重新实机验收。
