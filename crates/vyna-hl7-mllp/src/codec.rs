//! Streaming MLLP codec for `tokio_util::codec::Framed`
//!
//! [`MllpCodec`] implements [`Decoder`] and [`Encoder`] for MLLP framing
//! (`<VT>payload<FS><CR>`), so any `AsyncRead + AsyncWrite` transport can be
//! driven as a backpressure-aware stream/sink of HL7 messages:
//!
//! ```no_run
//! use tokio::net::TcpStream;
//! use tokio_util::codec::Framed;
//! use vyna_hl7_mllp::codec::{MllpCodec, MllpFramed};
//!
//! # async fn run() -> std::io::Result<()> {
//! let stream = TcpStream::connect("127.0.0.1:2575").await?;
//! let framed: MllpFramed<TcpStream> = Framed::new(stream, MllpCodec::new());
//! // Drive `framed` with futures `SinkExt::send` / `StreamExt::next`.
//! # Ok(())
//! # }
//! ```
//!
//! Decoding mirrors [`crate::MllpClient::receive_message`]: bytes before the
//! first `VT` are discarded as line noise, the frame ends at the first
//! `FS CR` pair, and frames larger than the configured limit are rejected.

use bytes::{BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder, Framed};

use super::{CARRIAGE_RETURN, DEFAULT_MAX_MESSAGE_SIZE, END_OF_BLOCK, START_OF_BLOCK};

/// `Framed` transport speaking MLLP on both halves.
pub type MllpFramed<T> = Framed<T, MllpCodec>;

/// Errors raised while framing or deframing MLLP bytes.
#[derive(Debug, thiserror::Error)]
pub enum MllpCodecError {
    /// A frame (markers included) exceeded the configured size limit.
    #[error("message exceeds maximum size of {0} bytes")]
    MessageTooLarge(usize),
    /// The stream ended before the closing `FS CR` pair arrived.
    #[error("connection closed mid-frame")]
    TruncatedFrame,
    /// Underlying transport I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// MLLP frame codec: `<VT>payload<FS><CR>`.
///
/// Decodes to the raw payload bytes (frame markers stripped); callers parse
/// with [`vyna_hl7_parser::parse_message`] after applying their charset.
#[derive(Debug, Clone)]
pub struct MllpCodec {
    max_message_size: usize,
}

impl MllpCodec {
    /// Codec accepting frames up to [`DEFAULT_MAX_MESSAGE_SIZE`] bytes.
    pub fn new() -> Self {
        Self {
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
        }
    }

    /// Codec with an explicit frame-size limit (markers included).
    pub fn with_max_message_size(max_message_size: usize) -> Self {
        Self { max_message_size }
    }

    /// Current frame-size limit in bytes.
    pub fn max_message_size(&self) -> usize {
        self.max_message_size
    }

    fn encode_payload(&self, payload: &[u8], dst: &mut BytesMut) -> Result<(), MllpCodecError> {
        if payload.len() + 3 > self.max_message_size {
            return Err(MllpCodecError::MessageTooLarge(self.max_message_size));
        }
        dst.reserve(payload.len() + 3);
        dst.put_u8(START_OF_BLOCK);
        dst.put_slice(payload);
        dst.put_u8(END_OF_BLOCK);
        dst.put_u8(CARRIAGE_RETURN);
        Ok(())
    }
}

impl Default for MllpCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for MllpCodec {
    type Item = BytesMut;
    type Error = MllpCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<BytesMut>, MllpCodecError> {
        // Discard line noise before the first start-of-block.
        let start = match src.iter().position(|b| *b == START_OF_BLOCK) {
            Some(pos) => pos,
            None => {
                src.clear();
                return Ok(None);
            }
        };
        if start > 0 {
            let _ = src.split_to(start);
        }

        // Scan for the first FS immediately followed by CR.
        let mut idx = 1;
        while idx + 1 < src.len() {
            if src[idx] == END_OF_BLOCK && src[idx + 1] == CARRIAGE_RETURN {
                let frame_len = idx + 2;
                if frame_len > self.max_message_size {
                    return Err(MllpCodecError::MessageTooLarge(self.max_message_size));
                }
                let mut frame = src.split_to(frame_len);
                let _ = frame.split_to(1); // strip VT
                frame.truncate(frame.len() - 2); // strip FS CR
                return Ok(Some(frame));
            }
            idx += 1;
        }

        if src.len() > self.max_message_size {
            return Err(MllpCodecError::MessageTooLarge(self.max_message_size));
        }
        // Avoid re-scanning a huge partial frame one byte at a time.
        src.reserve(8192);
        Ok(None)
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<BytesMut>, MllpCodecError> {
        match self.decode(src)? {
            Some(frame) => Ok(Some(frame)),
            None if src.is_empty() => Ok(None),
            None => Err(MllpCodecError::TruncatedFrame),
        }
    }
}

impl Encoder<&[u8]> for MllpCodec {
    type Error = MllpCodecError;

    fn encode(&mut self, payload: &[u8], dst: &mut BytesMut) -> Result<(), MllpCodecError> {
        self.encode_payload(payload, dst)
    }
}

impl Encoder<Vec<u8>> for MllpCodec {
    type Error = MllpCodecError;

    fn encode(&mut self, payload: Vec<u8>, dst: &mut BytesMut) -> Result<(), MllpCodecError> {
        self.encode_payload(&payload, dst)
    }
}

impl Encoder<bytes::Bytes> for MllpCodec {
    type Error = MllpCodecError;

    fn encode(&mut self, payload: bytes::Bytes, dst: &mut BytesMut) -> Result<(), MllpCodecError> {
        self.encode_payload(&payload, dst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAYLOAD: &[u8] = b"MSH|^~\\&|S";

    fn framed_payload() -> BytesMut {
        let mut codec = MllpCodec::new();
        let mut buf = BytesMut::new();
        codec.encode(PAYLOAD, &mut buf).unwrap();
        buf
    }

    #[test]
    fn encode_wraps_payload() {
        let buf = framed_payload();
        assert_eq!(buf[0], START_OF_BLOCK);
        assert_eq!(&buf[buf.len() - 2..], &[END_OF_BLOCK, CARRIAGE_RETURN]);
        assert_eq!(&buf[1..buf.len() - 2], PAYLOAD);
    }

    #[test]
    fn encode_rejects_oversize_payload() {
        let mut codec = MllpCodec::with_max_message_size(8);
        let mut buf = BytesMut::new();
        let err = codec.encode(b"123456789".as_slice(), &mut buf).unwrap_err();
        assert!(matches!(err, MllpCodecError::MessageTooLarge(8)));
    }

    #[test]
    fn decode_whole_frame() {
        let mut codec = MllpCodec::new();
        let mut buf = framed_payload();
        let payload = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(payload.as_ref(), PAYLOAD);
        assert!(buf.is_empty());
    }

    #[test]
    fn decode_split_frame() {
        let mut codec = MllpCodec::new();
        let full = framed_payload();
        let mut buf = BytesMut::from(&full[..5]);
        assert!(codec.decode(&mut buf).unwrap().is_none());
        buf.extend_from_slice(&full[5..]);
        let payload = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(payload.as_ref(), PAYLOAD);
    }

    #[test]
    fn decode_skips_line_noise() {
        let mut codec = MllpCodec::new();
        let mut buf = BytesMut::from(&b"\x00\xffGARBAGE"[..]);
        assert!(codec.decode(&mut buf).unwrap().is_none());
        assert!(buf.is_empty());
        buf.extend_from_slice(&framed_payload());
        let payload = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(payload.as_ref(), PAYLOAD);
    }

    #[test]
    fn decode_multiple_frames_in_one_buffer() {
        let mut codec = MllpCodec::new();
        let frame = framed_payload();
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&frame);
        buf.extend_from_slice(&frame);
        assert_eq!(codec.decode(&mut buf).unwrap().unwrap().as_ref(), PAYLOAD);
        assert_eq!(codec.decode(&mut buf).unwrap().unwrap().as_ref(), PAYLOAD);
        assert!(codec.decode(&mut buf).unwrap().is_none());
    }

    #[test]
    fn decode_bare_fs_without_cr_is_payload() {
        let mut codec = MllpCodec::new();
        let mut buf = BytesMut::from(&b"\x0bAB\x1cX\x1c\x0d"[..]);
        let payload = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(payload.as_ref(), b"AB\x1cX");
    }

    #[test]
    fn decode_rejects_oversize_frame() {
        let mut codec = MllpCodec::with_max_message_size(8);
        let mut buf = framed_payload();
        let err = codec.decode(&mut buf).unwrap_err();
        assert!(matches!(err, MllpCodecError::MessageTooLarge(8)));
    }

    #[test]
    fn decode_eof_empty_is_clean() {
        let mut codec = MllpCodec::new();
        let mut buf = BytesMut::new();
        assert!(codec.decode_eof(&mut buf).unwrap().is_none());
    }

    #[test]
    fn decode_eof_partial_frame_errors() {
        let mut codec = MllpCodec::new();
        let mut buf = BytesMut::from(&b"\x0bMSH|partial"[..]);
        let err = codec.decode_eof(&mut buf).unwrap_err();
        assert!(matches!(err, MllpCodecError::TruncatedFrame));
    }

    #[tokio::test]
    async fn framed_roundtrip_over_duplex() {
        use futures::{SinkExt, StreamExt};

        let (client, server) = tokio::io::duplex(1024);
        let mut client: MllpFramed<_> = Framed::new(client, MllpCodec::new());
        let mut server: MllpFramed<_> = Framed::new(server, MllpCodec::new());

        client.send(PAYLOAD).await.unwrap();
        let received = server.next().await.unwrap().unwrap();
        assert_eq!(received.as_ref(), PAYLOAD);

        server.send(received.freeze()).await.unwrap();
        let echo = client.next().await.unwrap().unwrap();
        assert_eq!(echo.as_ref(), PAYLOAD);
    }

    #[tokio::test]
    async fn framed_split_writes_reassemble() {
        use futures::StreamExt;
        use tokio::io::AsyncWriteExt;

        let (client, server) = tokio::io::duplex(1024);
        let mut client = client;
        let mut server: MllpFramed<_> = Framed::new(server, MllpCodec::new());

        let frame = framed_payload();
        for chunk in frame.chunks(3) {
            client.write_all(chunk).await.unwrap();
        }
        let received = server.next().await.unwrap().unwrap();
        assert_eq!(received.as_ref(), PAYLOAD);
    }
}
