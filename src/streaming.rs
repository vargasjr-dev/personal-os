/// Streaming — Server-Sent Events (SSE) parser for Anthropic streaming.
///
/// When the kernel calls Claude with `stream: true`, the response
/// arrives as a series of SSE events. This module parses them into
/// typed events so the kernel can display text token-by-token.
///
/// SSE format:
///   event: content_block_delta
///   data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

use alloc::string::String;
use alloc::vec::Vec;
use serde::Deserialize;

/// A parsed SSE event from the Anthropic stream.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Stream has started — message metadata.
    MessageStart { id: String, model: String },
    /// A new content block began.
    ContentBlockStart { index: usize },
    /// A text delta — the actual streamed tokens.
    TextDelta { index: usize, text: String },
    /// A content block finished.
    ContentBlockStop { index: usize },
    /// The entire message is done.
    MessageDone { stop_reason: String },
    /// Ping/keepalive — ignore.
    Ping,
    /// Unknown or unparseable event.
    Unknown(String),
}

/// Raw SSE data payloads from Anthropic.
#[derive(Debug, Deserialize)]
struct SseData {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    index: usize,
    #[serde(default)]
    delta: Option<DeltaPayload>,
    #[serde(default)]
    message: Option<MessagePayload>,
}

#[derive(Debug, Deserialize)]
struct DeltaPayload {
    #[serde(rename = "type")]
    delta_type: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessagePayload {
    id: String,
    model: String,
}

/// Parse a single SSE line pair (event + data) into a StreamEvent.
///
/// SSE format is:
///   event: <event_type>\n
///   data: <json>\n\n
pub fn parse_sse_event(event_type: &str, data: &str) -> StreamEvent {
    match event_type {
        "ping" => StreamEvent::Ping,
        "message_start" => {
            if let Ok(parsed) = crate::json::from_str::<SseData>(data) {
                if let Some(msg) = parsed.message {
                    return StreamEvent::MessageStart {
                        id: msg.id,
                        model: msg.model,
                    };
                }
            }
            StreamEvent::Unknown(String::from(data))
        }
        "content_block_start" => {
            if let Ok(parsed) = crate::json::from_str::<SseData>(data) {
                return StreamEvent::ContentBlockStart {
                    index: parsed.index,
                };
            }
            StreamEvent::Unknown(String::from(data))
        }
        "content_block_delta" => {
            if let Ok(parsed) = crate::json::from_str::<SseData>(data) {
                if let Some(delta) = parsed.delta {
                    if delta.delta_type == "text_delta" {
                        return StreamEvent::TextDelta {
                            index: parsed.index,
                            text: delta.text,
                        };
                    }
                }
            }
            StreamEvent::Unknown(String::from(data))
        }
        "content_block_stop" => {
            if let Ok(parsed) = crate::json::from_str::<SseData>(data) {
                return StreamEvent::ContentBlockStop {
                    index: parsed.index,
                };
            }
            StreamEvent::Unknown(String::from(data))
        }
        "message_delta" => {
            if let Ok(parsed) = crate::json::from_str::<SseData>(data) {
                if let Some(delta) = parsed.delta {
                    return StreamEvent::MessageDone {
                        stop_reason: delta.stop_reason.unwrap_or_default(),
                    };
                }
            }
            StreamEvent::Unknown(String::from(data))
        }
        "message_stop" => StreamEvent::MessageDone {
            stop_reason: String::from("end_turn"),
        },
        _ => StreamEvent::Unknown(String::from(event_type)),
    }
}

/// Parse a raw SSE response body into a vector of events.
///
/// Handles the full `event: ...\ndata: ...\n\n` format.
pub fn parse_stream(body: &str) -> Vec<StreamEvent> {
    let mut events = Vec::new();
    let mut current_event = String::new();
    let mut current_data = String::new();

    for line in body.lines() {
        if let Some(event) = line.strip_prefix("event: ") {
            current_event = String::from(event.trim());
        } else if let Some(data) = line.strip_prefix("data: ") {
            current_data = String::from(data.trim());
        } else if line.is_empty() && !current_event.is_empty() {
            events.push(parse_sse_event(&current_event, &current_data));
            current_event.clear();
            current_data.clear();
        }
    }

    // Handle trailing event without final blank line
    if !current_event.is_empty() {
        events.push(parse_sse_event(&current_event, &current_data));
    }

    events
}

/// Extract all text from a stream of events (concatenate TextDeltas).
pub fn extract_text(events: &[StreamEvent]) -> String {
    let mut text = String::new();
    for event in events {
        if let StreamEvent::TextDelta { text: t, .. } = event {
            text.push_str(t);
        }
    }
    text
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_parse_text_delta() {
        let event = parse_sse_event(
            "content_block_delta",
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#,
        );
        match event {
            StreamEvent::TextDelta { index, text } => {
                assert_eq!(index, 0);
                assert_eq!(text, "Hello");
            }
            _ => panic!("Expected TextDelta"),
        }
    }

    #[test_case]
    fn test_parse_stream() {
        let body = "event: ping\ndata: {}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}\n\nevent: message_stop\ndata: {}\n\n";
        let events = parse_stream(body);
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], StreamEvent::Ping));
        assert_eq!(extract_text(&events), "Hi");
    }

    #[test_case]
    fn test_extract_text() {
        let events = vec![
            StreamEvent::Ping,
            StreamEvent::TextDelta { index: 0, text: String::from("Hello ") },
            StreamEvent::TextDelta { index: 0, text: String::from("world!") },
            StreamEvent::MessageDone { stop_reason: String::from("end_turn") },
        ];
        assert_eq!(extract_text(&events), "Hello world!");
    }
}
