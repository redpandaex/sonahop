use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time::{Duration, timeout};

pub(crate) const VERSION: u8 = 3;
pub(crate) const VERSION_ERROR: &str = "连接协议版本不兼容，请将两端更新到同一版本";
pub(crate) const MAX_UDP: usize = 1024;
pub(crate) const MAX_CONTROL: usize = 2048;
const MAX_FRAME: usize = 4096;
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Discovery {
    Probe {
        version: u8,
        nonce: String,
    },
    Reply {
        version: u8,
        nonce: String,
        id: String,
        name: String,
        port: u16,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Frame {
    Hello {
        version: u8,
        name: String,
        ephemeral_key: String,
    },
    Auth {
        version: u8,
        id: String,
        host_name: String,
        ephemeral_key: String,
    },
    Rejected {
        version: u8,
        reason: String,
    },
    Encrypted {
        version: u8,
        counter: u64,
        body: String,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Payload {
    Confirm { id: String },
    Accepted { id: String, host_name: String },
    Rejected { reason: String },
    Ping,
    Pong,
    Bye,
    Control { payload: Value },
    Media { payload: String },
}
pub(crate) fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.chars().count() <= 64 && !name.chars().any(char::is_control)
}
pub(crate) fn local_ip(address: Ipv4Addr) -> bool {
    (address.is_private() || address.is_link_local() || address.is_loopback())
        && !address.is_unspecified()
        && !address.is_multicast()
        && !address.is_broadcast()
}
pub(crate) fn local_target(address: &str, port: u16) -> Result<SocketAddrV4, String> {
    let address = address
        .trim()
        .parse::<Ipv4Addr>()
        .map_err(|_| "请输入局域网 IPv4 地址，不支持域名")?;
    if !local_ip(address) {
        return Err("只允许私有、链路本地或回环 IPv4 主机地址".into());
    }
    if port == 0 {
        return Err("端口应为 1–65535".into());
    }
    Ok(SocketAddrV4::new(address, port))
}
pub(crate) fn valid_id(id: &str) -> bool {
    id.len() == 36 && uuid::Uuid::parse_str(id).is_ok()
}

struct BoundedJson {
    data: Vec<u8>,
    limit: usize,
}
impl Write for BoundedJson {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if data.len() > self.limit.saturating_sub(self.data.len()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "JSON size limit",
            ));
        }
        self.data.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub(crate) fn bounded_json<T: Serialize>(value: &T, limit: usize) -> Result<Vec<u8>, String> {
    let mut writer = BoundedJson {
        data: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut writer, value).map_err(|_| "控制消息格式或长度超出限制")?;
    Ok(writer.data)
}
pub(crate) fn validate_control(payload: &Value) -> Result<(), String> {
    bounded_json(payload, MAX_CONTROL).map(|_| ())
}

fn decode_frame(payload: &[u8]) -> Result<Frame, String> {
    // Inspect the wire version before strict v3 fields: v2 has a different key field.
    #[derive(Deserialize)]
    struct FrameVersion {
        version: u64,
    }
    let envelope: FrameVersion =
        serde_json::from_slice(payload).map_err(|_| "控制帧格式不正确".to_owned())?;
    if envelope.version != u64::from(VERSION) {
        return Err(VERSION_ERROR.into());
    }
    serde_json::from_slice(payload).map_err(|_| "控制帧格式不正确".to_owned())
}

/// read() is cancellation-safe; partial prefixes and bodies survive select! ticks.
/// Announced length is rejected before any payload allocation.
pub(crate) struct FrameReader<R> {
    inner: R,
    header: [u8; 4],
    header_read: usize,
    payload: Vec<u8>,
    payload_read: usize,
}
impl<R: AsyncRead + Unpin> FrameReader<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            header: [0; 4],
            header_read: 0,
            payload: Vec::new(),
            payload_read: 0,
        }
    }
    pub async fn read(&mut self) -> Result<Frame, String> {
        while self.header_read < 4 {
            let size = self
                .inner
                .read(&mut self.header[self.header_read..])
                .await
                .map_err(|error| format!("连接读取失败：{error}"))?;
            if size == 0 {
                return Err("连接已关闭".into());
            }
            self.header_read += size;
        }
        let length = u32::from_be_bytes(self.header) as usize;
        if length == 0 || length > MAX_FRAME {
            return Err("控制帧长度超出限制".into());
        }
        if self.payload.is_empty() {
            self.payload.resize(length, 0);
        }
        while self.payload_read < length {
            let size = self
                .inner
                .read(&mut self.payload[self.payload_read..])
                .await
                .map_err(|error| format!("连接读取失败：{error}"))?;
            if size == 0 {
                return Err("连接已关闭".into());
            }
            self.payload_read += size;
        }
        let frame = decode_frame(&self.payload);
        self.header_read = 0;
        self.payload.clear();
        self.payload_read = 0;
        frame
    }
}
pub(crate) async fn write<W: AsyncWrite + Unpin>(
    writer: &mut W,
    frame: &Frame,
) -> Result<(), String> {
    let payload = bounded_json(frame, MAX_FRAME)?;
    timeout(Duration::from_secs(2), async {
        writer
            .write_all(&(payload.len() as u32).to_be_bytes())
            .await?;
        writer.write_all(&payload).await
    })
    .await
    .map_err(|_| "连接连续发送阻塞超过 2 秒，请检查网络；已结束会话".to_owned())?
    .map_err(|error| format!("连接写入失败：{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_handshake_reports_version_before_changed_key_fields() {
        for payload in [
            br#"{"type":"hello","version":2,"name":"player","spake":"old"}"#.as_slice(),
            br#"{"type":"auth","version":2,"id":"old-id","hostName":"host","spake":"old"}"#
                .as_slice(),
        ] {
            assert!(matches!(decode_frame(payload), Err(error) if error == VERSION_ERROR));
        }
    }

    #[test]
    fn v3_frames_require_the_new_key_field_and_reject_unknown_fields() {
        assert!(matches!(
            decode_frame(br#"{"type":"hello","version":3,"name":"player","ephemeralKey":"key"}"#).unwrap(),
            Frame::Hello { version: VERSION, ephemeral_key, .. } if ephemeral_key == "key"
        ));
        for payload in [
            br#"{"type":"hello","version":3,"name":"player","spake":"old"}"#.as_slice(),
            br#"{"type":"hello","version":3,"name":"player","ephemeralKey":"key","spake":"old"}"#
                .as_slice(),
            br#"{"type":"hello","name":"player","ephemeralKey":"key"}"#.as_slice(),
        ] {
            assert!(decode_frame(payload).is_err());
        }
    }
}
