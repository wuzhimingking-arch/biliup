use super::*;
use serde_json::json;
use std::io::Write;

#[test]
fn cookie_identity_is_used_in_registration_when_context_uid_is_missing() {
    let context = PlatformContext::new()
        .with_cookie("SESSDATA=fixture-secret; DedeUserID = 12345; bili_jct=fixture-csrf");
    let uid = Bilibili::auth_uid(&context).unwrap();
    let packet = Bilibili::build_auth_packet(uid, 67890, "fixture-token");
    let body: Value = serde_json::from_slice(&packet[16..]).unwrap();
    assert_eq!(body["uid"], 12345);
    assert_eq!(body["roomid"], 67890);
    assert_eq!(body["protover"], 3);
    assert_eq!(Bilibili::auth_uid(&PlatformContext::new()).unwrap(), 0);
    assert_eq!(Bilibili::auth_uid(&context.with_uid(12345)).unwrap(), 12345);
}

#[test]
fn invalid_or_mismatched_cookie_identity_fails_without_disclosing_credentials() {
    for context in [
        PlatformContext::new().with_cookie("SESSDATA=fixture-secret"),
        PlatformContext::new().with_cookie("DedeUserID=fixture-secret"),
        PlatformContext::new().with_cookie("DedeUserID=0"),
        PlatformContext::new().with_cookie("DedeUserID=1; DedeUserID=2"),
        PlatformContext::new()
            .with_cookie("SESSDATA=fixture-secret; DedeUserID=1")
            .with_uid(0),
    ] {
        let error = Bilibili::auth_uid(&context).unwrap_err().to_string();
        assert!(!error.contains("fixture-secret"));
    }
}

#[test]
fn connection_info_retains_valid_unique_nodes_and_requires_token() {
    let info = json!({"code":0,"data":{"token":"fixture-secret","host_list":[
        {"host":"a.example","wss_port":2245}, {"host":"b.example","wss_port":443},
        {"host":"a.example","wss_port":2245}, {"host":"bad/path","wss_port":443},
        {"host":"bad.example","wss_port":0}
    ]}});
    let (urls, token) = Bilibili::parse_danmu_info(&info).unwrap();
    assert_eq!(
        urls,
        ["wss://a.example:2245/sub", "wss://b.example:443/sub"]
    );
    assert_eq!(token, "fixture-secret");
    for bad in [
        json!({"code":-352}),
        json!({"code":0,"data":{"token":""}}),
        json!({"code":0,"data":{"token":"fixture-secret","host_list":[]}}),
    ] {
        let error = Bilibili::parse_danmu_info(&bad).unwrap_err().to_string();
        assert!(!error.contains("fixture-secret"));
    }
}

#[test]
fn authentication_codes_are_observable_and_malformed_response_is_rejected() {
    let platform = Bilibili::new();
    for code in [0, -101, -102] {
        let bytes = build_packet(format!("{{\"code\":{code}}}").as_bytes(), 8);
        let result = platform.decode_message(&bytes).unwrap();
        assert_eq!(result.auth_code, Some(code));
        assert_eq!(result.packet_count, 1);
        assert!(result.events.is_empty());
    }
    assert!(platform.decode_message(&build_packet(b"{}", 8)).is_err());
}

#[test]
fn heartbeat_response_and_extended_header_are_decoded() {
    let platform = Bilibili::new();
    let mut bytes = build_packet(&123u32.to_be_bytes(), 3);
    assert_eq!(
        platform.decode_message(&bytes).unwrap().heartbeat_replies,
        1
    );
    bytes[4..6].copy_from_slice(&15u16.to_be_bytes());
    assert_eq!(platform.decode_message(&bytes).unwrap().packet_count, 0);
    let mut extended = build_packet(b"{}", 8);
    extended.splice(16..16, [0u8; 4]);
    let len = extended.len() as u32;
    extended[..4].copy_from_slice(&len.to_be_bytes());
    extended[4..6].copy_from_slice(&20u16.to_be_bytes());
    assert_eq!(Bilibili::decode_packet(&extended)[0].body, b"{}");
}

#[test]
fn brotli_notification_preserves_real_event_fields() {
    let raw = build_packet(
        br#"{"cmd":"DANMU_MSG","info":[[0,1,25,16777215],"brotli fixture",[42,"fixture"]]}"#,
        5,
    );
    let mut compressed = Vec::new();
    {
        let mut writer = brotli::CompressorWriter::new(&mut compressed, 4096, 5, 22);
        writer.write_all(&raw).unwrap();
    }
    let mut frame = build_packet(&compressed, 5);
    frame[6..8].copy_from_slice(&3u16.to_be_bytes());
    let result = Bilibili::new().decode_message(&frame).unwrap();
    assert_eq!(result.events.len(), 1);
    let DanmakuEvent::Chat(chat) = &result.events[0] else {
        panic!("chat expected")
    };
    assert_eq!(chat.content, "brotli fixture");
    assert_eq!(chat.uid, Some(42));
}
