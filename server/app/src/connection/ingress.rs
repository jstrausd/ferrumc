//! A finite FIFO fence between authoritative outputs and buffered client input.

use std::collections::VecDeque;

use bytes::Bytes;
use ferrumc_codec::BoundedReader;
use ferrumc_proto::generated::play::ServerboundKeepAlive;
use ferrumc_session::{OutboundMessage, PlayerSessionHandle, DEFAULT_OUTBOUND_CAPACITY};

/// At most one frame and one finite prefix of the existing bounded output FIFO.
///
/// The prefix is never extended by a producer. Each message takes a normal
/// output/flush turn; then the held frame runs before a new prefix can start.
/// App constructs the router with `DEFAULT_OUTBOUND_CAPACITY`. Retention adds at
/// most 256 messages and one already frame-limited body; byte sizes vary.
#[derive(Default)]
pub(super) struct IngressFence {
    body: Option<Bytes>,
    prefix: VecDeque<OutboundMessage>,
}

impl IngressFence {
    /// Freezes the ready FIFO prefix once. An exact Keep Alive body remains
    /// connection-only control work and needs no positional ordering fence.
    pub(super) fn hold(&mut self, body: Bytes, handle: &mut PlayerSessionHandle) {
        self.body = Some(body);
        for _ in 0..DEFAULT_OUTBOUND_CAPACITY {
            let Some(message) = handle.try_recv() else {
                break;
            };
            self.prefix.push_back(message);
        }
    }

    /// Takes the held frame after every message in its finite prefix was sent.
    pub(super) fn take_ready_body(&mut self) -> Option<Bytes> {
        if self.prefix.is_empty() {
            self.body.take()
        } else {
            None
        }
    }

    /// Whether socket reads/input dispatch must yield to the frozen prefix.
    pub(super) fn is_fenced(&self) -> bool {
        !self.prefix.is_empty()
    }

    /// Delivers one retained output; called only by the selected output turn.
    pub(super) fn pop_outbound(&mut self) -> Option<OutboundMessage> {
        self.prefix.pop_front()
    }
}

/// A Keep Alive bypass is safe only for the exact complete packet body. Invalid
/// bodies still use normal strict dispatch and cannot extend a response clock.
pub(super) fn is_exact_keep_alive(body: &[u8]) -> bool {
    let mut reader = BoundedReader::new(body);
    reader.read_var_int() == Ok(ServerboundKeepAlive::PACKET_ID)
        && ServerboundKeepAlive::decode(&mut reader).is_ok()
        && reader.remaining() == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn control_bypass_rejects_partial_trailing_and_non_keep_alive_bodies() {
        let mut body = BytesMut::new();
        ServerboundKeepAlive::new(7)
            .encode(&mut body)
            .expect("echo");
        assert!(is_exact_keep_alive(&body));
        for length in 0..body.len() {
            assert!(!is_exact_keep_alive(&body[..length]));
        }
        body.extend_from_slice(&[0]);
        assert!(!is_exact_keep_alive(&body));
        assert!(!is_exact_keep_alive(&[0, 1]));
    }
}
