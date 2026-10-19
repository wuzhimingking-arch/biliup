# 定制分支维护规范

- 本分支基于官方 biliup v1.2.11 `3d884a570be7761e65413785cfc63d88a97ef0e5`，保留官方完整历史与许可证。
- Rust workspace、Tokio/tokio-tungstenite 与 Next.js WebUI；复用现有架构，不重写其他平台或投稿逻辑。
- 修改前阅读 PROJECT_CONTEXT.md；开发完成更新相关事实、追加 DEV_LOG.md，并使用中文提交。
- 测试模式仅在 `BILIUP_LOCAL_DANMAKU_TEST=1` 时启用；正式环境使用未设置或 0，不默认禁用投稿/钩子。
- 必须检查受影响 Rust 文件格式，运行弹幕协议/XML 回归、正式模式保护测试、必要构建及敏感信息扫描。验证入口见 tools/danmaku/README.md。
- 凭据、数据库、录像、真实弹幕、日志、镜像和现场生产配置不得进入提交或镜像构建上下文。测试 fixture 必须明确标注。
- SQLite 迁移保持 LF 和原始内容；不修改既有迁移校验和。
- origin 为个人 Fork，upstream 为官方仓库。保留独立定制分支，不强推，不向官方仓库推送或自动创建 PR。
- 同步新版先在独立分支检查冲突并回归；不能用旧文件覆盖新版。默认分支保持官方状态，未经验证不合并定制分支。
