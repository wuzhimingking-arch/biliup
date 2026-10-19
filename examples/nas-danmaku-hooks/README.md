# 外部弹幕压制对接示例

这些脚本是独立队列服务的可选适配示例，不是 biliup 内置 worker，也不包含压制容器或生产配置。只在已有兼容外部服务时使用；默认不启用。

将 finalized 视频与同名 XML 两行路径传入 safe hook，要求 `/opt` 工作目录内的常规 FLV/XML。外部 worker 必须原子领取 pending job，写 results 中 `.ok`/`.err`，编码成功后保留原视频备份并原子替换目标；路径映射由部署者自行核对。没有兼容 worker 就不要启用。

配置字段已经表示命令，不要把 `run =` 再写进命令正文：

```json
[{"run":"sh /opt/burn-hook-safe.sh"}]
```

缺失/空/未闭合 XML 或明确压制失败时，safe hook 先检查视频流与首尾 2 秒解码，通过才允许后续使用原片。损坏视频阻挡；未领取任务 120 秒原子撤回；已领取编码任务保留 48 小时上限，截止时仍不放行，避免编码器与上传争用。首尾检查不是全片损坏审计。

两个脚本放入自己的数据目录，再按需设置对应房间钩子；不会自动修改数据库或直播间。保留原文件与数据库备份，不能照抄其他人的 NAS 路径。主服务部署示例只启动 biliup，不创建外部 worker。

隔离回归不访问网络、不投稿，使用合成短片与明确标注的 fixture XML，不能代替真实弹幕验收：

```sh
BILIUP_TEST_IMAGE=biliup-custom:danmaku-runtime python3 examples/nas-danmaku-hooks/test-hooks.py
```
