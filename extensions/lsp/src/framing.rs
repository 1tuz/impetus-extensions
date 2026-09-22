//! LSP stdio framing: `Content-Length: N\r\n\r\n` + JSON body.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub async fn write_message<W: AsyncWrite + Unpin>(writer: &mut W, body: &Value) -> Result<()> {
    let payload = serde_json::to_vec(body).context("encode LSP JSON")?;
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(&payload).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_message<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Value> {
    let mut header = Vec::with_capacity(64);
    let mut buf = [0u8; 1];
    loop {
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            bail!("LSP stdout closed while reading header");
        }
        header.push(buf[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
        if header.len() > 8192 {
            bail!("LSP header too large");
        }
    }

    let header_str = String::from_utf8_lossy(&header);
    let mut content_length = None;
    for line in header_str.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = Some(
                rest.trim()
                    .parse::<usize>()
                    .with_context(|| format!("bad Content-Length: {rest}"))?,
            );
        }
    }
    let len = content_length.ok_or_else(|| anyhow!("LSP message missing Content-Length"))?;
    let mut body = vec![0u8; len];
    reader
        .read_exact(&mut body)
        .await
        .context("read LSP body")?;
    serde_json::from_slice(&body).context("parse LSP JSON body")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::io::BufReader;

    #[tokio::test]
    async fn roundtrip_framed_message() {
        let mut buf = Vec::new();
        write_message(&mut buf, &json!({"jsonrpc":"2.0","id":1,"result":{}}))
            .await
            .unwrap();
        let mut reader = BufReader::new(buf.as_slice());
        let msg = read_message(&mut reader).await.unwrap();
        assert_eq!(msg["id"], 1);
    }
}
