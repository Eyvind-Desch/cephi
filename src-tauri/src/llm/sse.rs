//! Minimal Server-Sent Events reader for streaming APIs.

use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

/// What the event handler wants to happen next.
pub enum Flow {
    Continue,
    Stop,
}

pub enum ReadOutcome {
    Finished,
    Cancelled,
}

/// Calls `on_data` with the payload of every `data:` line until the stream ends, the handler
/// stops, or `cancel` fires. Events spanning several `data:` lines are joined with newlines.
pub async fn read<E>(
    response: reqwest::Response,
    cancel: &CancellationToken,
    mut on_data: impl FnMut(&str) -> Result<Flow, E>,
    map_transport: impl Fn(reqwest::Error) -> E,
) -> Result<ReadOutcome, E> {
    let mut stream = response.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();
    let mut data = String::new();

    loop {
        let next = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Ok(ReadOutcome::Cancelled),
            next = stream.next() => next,
        };
        let Some(chunk) = next else {
            // Some servers close without a trailing blank line.
            if !data.is_empty() {
                on_data(&data)?;
            }
            return Ok(ReadOutcome::Finished);
        };
        buffer.extend_from_slice(&chunk.map_err(&map_transport)?);

        while let Some(newline) = buffer.iter().position(|b| *b == b'\n') {
            let raw: Vec<u8> = buffer.drain(..=newline).collect();
            let line = String::from_utf8_lossy(&raw);
            let line = line.trim_end_matches(['\r', '\n']);

            if line.is_empty() {
                // Blank line: event complete.
                if !data.is_empty() {
                    let flow = on_data(&data)?;
                    data.clear();
                    if matches!(flow, Flow::Stop) {
                        return Ok(ReadOutcome::Finished);
                    }
                }
            } else if let Some(payload) = line.strip_prefix("data:") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(payload.strip_prefix(' ').unwrap_or(payload));
            }
            // `event:`, `id:`, `retry:` and comments are not needed: payloads carry their type.
        }
    }
}
