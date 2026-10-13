//! SSE framing stays in bytes until a blank line completes an event.
// Bound an event, including unfinished lines and ignored fields, before allocation grows.
pub(super) const EVENT_BYTE_LIMIT: usize = 8 * 1024 * 1024;
#[derive(Default)]
pub(super) struct SseDecoder {
    line: Vec<u8>,
    event: Vec<u8>,
    skip_lf: bool,
    started: bool,
}

impl SseDecoder {
    /// `true` means the consumer accepted a terminal event. Ignore its trailing bytes.
    pub(super) fn push(
        &mut self,
        bytes: &[u8],
        mut on_data: impl FnMut(&str) -> Result<bool, String>,
    ) -> Result<bool, String> {
        for &byte in bytes {
            if self.skip_lf {
                self.skip_lf = false;
                if byte == b'\n' {
                    continue;
                }
            }
            if byte != b'\r' && byte != b'\n' {
                if self.line.len() + self.event.len() >= EVENT_BYTE_LIMIT {
                    return Err("SSE event exceeds the 8 MiB limit".into());
                }
                self.line.push(byte);
                continue;
            }
            self.skip_lf = byte == b'\r';
            let mut line = std::mem::take(&mut self.line);
            if !self.started {
                self.started = true;
                if line.starts_with(&[0xef, 0xbb, 0xbf]) {
                    line.drain(..3);
                }
            }
            if !line.is_empty() {
                if self.event.len() + line.len() + 1 > EVENT_BYTE_LIMIT {
                    return Err("SSE event exceeds the 8 MiB limit".into());
                }
                self.event.extend_from_slice(&line);
                self.event.push(b'\n');
                continue;
            }
            let event = std::mem::take(&mut self.event);
            let text =
                std::str::from_utf8(&event).map_err(|e| format!("Invalid SSE UTF-8: {e}"))?;
            let mut data = Vec::new();
            for line in text.lines() {
                if line.starts_with(':') {
                    continue;
                }
                let (field, value) = line.split_once(':').unwrap_or((line, ""));
                if field == "data" {
                    data.push(value.strip_prefix(' ').unwrap_or(value));
                }
            }
            if !data.is_empty() {
                let data = data.join("\n");
                if on_data(&data)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// EOF never implicitly dispatches a pending event or completes an AI result.
    pub(super) fn eof_error(&self) -> String {
        if !self.line.is_empty() || !self.event.is_empty() {
            "Stream ended with an incomplete SSE event before [DONE]".into()
        } else {
            "Stream ended before [DONE]".into()
        }
    }
}
