use super::*;
use crate::protocols::bilibili::Bilibili;
use async_trait::async_trait;
use tokio::net::TcpListener;
use tokio_tungstenite::{WebSocketStream, accept_async};

struct MockBilibili(ConnectionInfo);
#[async_trait]
impl Platform for MockBilibili {
    fn name(&self) -> &'static str {
        "Bilibili"
    }
    async fn get_connection_info(&self, _: &str, _: &PlatformContext) -> Result<ConnectionInfo> {
        Ok(self.0.clone())
    }
    fn heartbeat_config(&self) -> crate::protocols::HeartbeatConfig {
        crate::protocols::HeartbeatConfig::binary(packet(2, b""), Duration::from_secs(30))
    }
    fn decode_message(&self, data: &[u8]) -> Result<DecodeResult> {
        Bilibili::new().decode_message(data)
    }
}

fn packet(op: u32, body: &[u8]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&((16 + body.len()) as u32).to_be_bytes());
    data.extend_from_slice(&16u16.to_be_bytes());
    data.extend_from_slice(&1u16.to_be_bytes());
    data.extend_from_slice(&op.to_be_bytes());
    data.extend_from_slice(&1u32.to_be_bytes());
    data.extend_from_slice(body);
    data
}

async fn listener() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}/sub", listener.local_addr().unwrap());
    (listener, url)
}

async fn accept(listener: &TcpListener) -> WebSocketStream<TcpStream> {
    accept_async(listener.accept().await.unwrap().0)
        .await
        .unwrap()
}

fn recorder(url: String, fallbacks: Vec<String>, timeout: Duration) -> (DanmakuRecorder, PathBuf) {
    let mut info = ConnectionInfo::new(url)
        .with_fallback_ws_urls(fallbacks)
        .with_registration(vec![RegistrationData::Binary(packet(7, b"{}"))]);
    info.auth_timeout = Some(timeout);
    let dir = std::env::temp_dir().join(format!(
        "biliup-mock-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    ));
    let config = RecorderConfig::new("https://live.bilibili.com/1", dir.join("record.xml"));
    (
        DanmakuRecorder {
            config,
            platform: Arc::new(MockBilibili(info)),
        },
        dir,
    )
}

async fn authenticate(ws: &mut WebSocketStream<TcpStream>) {
    let msg = ws.next().await.unwrap().unwrap().into_data();
    assert_eq!(u32::from_be_bytes(msg[8..12].try_into().unwrap()), 7);
    assert!(
        tokio::time::timeout(Duration::from_millis(60), ws.next())
            .await
            .is_err(),
        "heartbeat must wait for authentication"
    );
    ws.send(Message::Binary(packet(8, b"{\"code\":0}").into()))
        .await
        .unwrap();
    let heartbeat = ws.next().await.unwrap().unwrap().into_data();
    assert_eq!(u32::from_be_bytes(heartbeat[8..12].try_into().unwrap()), 2);
}

#[tokio::test]
async fn authenticated_recording_flushes_closes_xml_and_leaves_no_temporary_file() {
    let (listener, url) = listener().await;
    let (recorder, dir) = recorder(url, vec![], Duration::from_secs(2));
    let (tx, rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener).await;
        authenticate(&mut ws).await;
        ws.send(Message::Binary(
            packet(
                5,
                br#"{"cmd":"DANMU_MSG","info":[[0,1,25,16777215],"mock <&> chat",[1,"fixture"]]}"#,
            )
            .into(),
        ))
        .await
        .unwrap();
        tx.send(()).unwrap();
        while let Some(msg) = ws.next().await {
            if matches!(msg, Ok(Message::Close(_))) {
                break;
            }
        }
    });
    let (cmd_tx, cmd_rx) = mpsc::channel(2);
    let (stop_tx, stop_rx) = watch::channel(false);
    let task = tokio::spawn(recorder.run(cmd_rx, stop_rx));
    tokio::time::timeout(Duration::from_secs(3), rx)
        .await
        .unwrap()
        .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    stop_tx.send(true).unwrap();
    task.await.unwrap().unwrap();
    server.await.unwrap();
    drop(cmd_tx);
    let xml = fs::read_to_string(dir.join("record.xml")).unwrap();
    assert!(xml.contains("mock &lt;&amp;&gt; chat"));
    assert!(xml.trim_end().ends_with("</i>"));
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
    fs::remove_dir_all(dir).unwrap();
}

async fn rejected_or_silent(code: Option<i64>) -> DanmakuError {
    let (listener, url) = listener().await;
    let (recorder, dir) = recorder(url, vec![], Duration::from_millis(150));
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener).await;
        ws.next().await.unwrap().unwrap();
        if let Some(code) = code {
            ws.send(Message::Binary(
                packet(8, format!("{{\"code\":{code}}}").as_bytes()).into(),
            ))
            .await
            .unwrap();
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    });
    let (_cmd_tx, mut cmd_rx) = mpsc::channel(2);
    let (_stop_tx, mut stop_rx) = watch::channel(false);
    let config = XmlWriterConfig::default();
    let mut xml = XmlWriter::new(dir.join("test.xml"), config.clone()).unwrap();
    let error = recorder
        .connect_and_run(&mut cmd_rx, &mut stop_rx, &mut xml, &config)
        .await
        .unwrap_err();
    xml.finish().unwrap();
    server.await.unwrap();
    fs::remove_dir_all(dir).unwrap();
    error
}

#[tokio::test]
async fn auth_rejection_is_an_error() {
    assert!(matches!(
        rejected_or_silent(Some(-101)).await,
        DanmakuError::Authentication(-101)
    ));
}

#[tokio::test]
async fn missing_auth_reply_times_out() {
    assert!(matches!(
        rejected_or_silent(None).await,
        DanmakuError::AuthenticationTimeout
    ));
}

#[tokio::test]
async fn reset_after_handshake_switches_to_backup_node() {
    let (first, url) = listener().await;
    let (backup, backup_url) = listener().await;
    let (recorder, dir) = recorder(url, vec![backup_url], Duration::from_secs(2));
    let server1 = tokio::spawn(async move {
        let mut ws = accept(&first).await;
        ws.next().await;
        drop(ws);
    });
    let (ready_tx, ready_rx) = oneshot::channel();
    let server2 = tokio::spawn(async move {
        let mut ws = accept(&backup).await;
        authenticate(&mut ws).await;
        ready_tx.send(()).unwrap();
        while let Some(msg) = ws.next().await {
            if matches!(msg, Ok(Message::Close(_))) {
                break;
            }
        }
    });
    let (_cmd_tx, cmd_rx) = mpsc::channel(2);
    let (stop_tx, stop_rx) = watch::channel(false);
    let task = tokio::spawn(recorder.run(cmd_rx, stop_rx));
    tokio::time::timeout(Duration::from_secs(4), ready_rx)
        .await
        .unwrap()
        .unwrap();
    stop_tx.send(true).unwrap();
    task.await.unwrap().unwrap();
    server1.await.unwrap();
    server2.await.unwrap();
    assert_eq!(
        fs::read_dir(&dir).unwrap().count(),
        0,
        "empty session is removed on clean stop"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn disconnected_session_reconnects_after_backoff() {
    let (listener, url) = listener().await;
    let (recorder, dir) = recorder(url, vec![], Duration::from_secs(2));
    let (ready_tx, ready_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        let mut first = accept(&listener).await;
        authenticate(&mut first).await;
        drop(first);
        let mut second = accept(&listener).await;
        authenticate(&mut second).await;
        ready_tx.send(()).unwrap();
        while let Some(msg) = second.next().await {
            if matches!(msg, Ok(Message::Close(_))) {
                break;
            }
        }
    });
    let (_cmd_tx, cmd_rx) = mpsc::channel(2);
    let (stop_tx, stop_rx) = watch::channel(false);
    let task = tokio::spawn(recorder.run(cmd_rx, stop_rx));
    tokio::time::timeout(Duration::from_secs(40), ready_rx)
        .await
        .unwrap()
        .unwrap();
    stop_tx.send(true).unwrap();
    task.await.unwrap().unwrap();
    server.await.unwrap();
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn xml_flush_is_visible_before_close_and_close_is_idempotent() {
    let dir = std::env::temp_dir().join(format!(
        "biliup-flush-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    ));
    let path = dir.join("flush.xml");
    let mut writer = XmlWriter::new(
        &path,
        XmlWriterConfig {
            save_interval: 0,
            ..XmlWriterConfig::default()
        },
    )
    .unwrap();
    writer
        .write_event(&DanmakuEvent::Chat(crate::message::ChatMessage::new(
            "flush-check".to_string(),
        )))
        .unwrap();
    assert!(fs::read_to_string(&path).unwrap().contains("flush-check"));
    assert_eq!(writer.message_count(), 1);
    writer.finalize().unwrap();
    writer.finalize().unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert_eq!(text.matches("</i>").count(), 1);
    drop(writer);
    fs::remove_dir_all(dir).unwrap();
}
