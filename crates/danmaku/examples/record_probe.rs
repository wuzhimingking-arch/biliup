//! Short probe using the production recorder; optional credentials stay in a local file.
use danmaku::{DanmakuRecorder, PlatformContext, RecorderConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .with_ansi(false)
        .init();
    let args: Vec<String> = std::env::args().collect();
    let url = args
        .get(1)
        .ok_or("usage: record_probe URL OUTPUT [SECONDS] [COOKIE_JSON] [infer|cookie|zero]")?;
    let output = args.get(2).ok_or("missing output path")?;
    let seconds: u64 = args.get(3).map(|s| s.parse()).transpose()?.unwrap_or(30);
    let mut context = PlatformContext::new();
    if let Some(path) = args.get(4) {
        let account: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
        let cookies = account["cookie_info"]["cookies"]
            .as_array()
            .ok_or("missing cookies")?;
        let cookie = cookies
            .iter()
            .filter_map(|c| Some(format!("{}={}", c["name"].as_str()?, c["value"].as_str()?)))
            .collect::<Vec<_>>()
            .join("; ");
        let uid = account["token_info"]["mid"]
            .as_u64()
            .ok_or("missing numeric account UID")?;
        if cookie.is_empty() || uid == 0 {
            return Err("missing account identity".into());
        }
        let mode = args.get(5).map(String::as_str).unwrap_or("infer");
        context = context.with_cookie(cookie);
        match mode {
            "infer" => {}
            "cookie" => context = context.with_uid(uid),
            "zero" => context = context.with_uid(0),
            _ => return Err("invalid UID mode".into()),
        }
        tracing::info!(
            uid_mode = mode,
            cookie_present = true,
            "Probe identity (credentials redacted)"
        );
    }
    let handle =
        DanmakuRecorder::new(RecorderConfig::new(url, output).with_context(context))?.start();
    tokio::time::sleep(Duration::from_secs(seconds)).await;
    handle.stop().await?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    Ok(())
}
