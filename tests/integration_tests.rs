use bytes::{BufMut, BytesMut};
use std::sync::{Arc, Mutex};
use vortex::error::Error;
use vortex::tlv::Tag;
use vortex::Vortex as Packet;

/// Mock peer for testing
pub struct MockPeer {
    pieces: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl MockPeer {
    pub fn new() -> Self {
        MockPeer {
            pieces: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_piece(&self, piece: Vec<u8>) {
        self.pieces.lock().unwrap().push(piece);
    }

    pub fn handle_packet(&self, packet: &Packet) -> Result<Packet, Error> {
        match packet.tag() {
            Tag::DownloadPiece => {
                let value = String::from_utf8(packet.to_bytes()[6..].to_vec())?;
                let parts: Vec<&str> = value.split('-').collect();
                if parts.len() != 2 {
                    return Err(Error::InvalidPacket(
                        "invalid download piece format".to_string(),
                    ));
                }

                let piece_id: usize = parts[1].parse()?;
                let pieces = self.pieces.lock().unwrap();
                if piece_id >= pieces.len() {
                    return Err(Error::InvalidPacket(format!(
                        "piece {} not found",
                        piece_id
                    )));
                }

                Ok(Packet::new(
                    Tag::PieceContent,
                    pieces[piece_id].clone().into(),
                )?)
            }
            _ => Err(Error::InvalidPacket("unexpected tag".to_string())),
        }
    }
}

impl Default for MockPeer {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn test_piece_download_flow() {
    let peer = MockPeer::new();
    let piece = vec![1, 2, 3, 4];
    peer.add_piece(piece.clone());

    // Create a download request for piece 0
    let task_id = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let request = Packet::new(
        Tag::DownloadPiece,
        format!("{}-0", task_id).into_bytes().into(),
    )
    .unwrap();

    // Handle the request
    let response = peer.handle_packet(&request).unwrap();

    // Verify the response
    assert_eq!(response.tag(), &Tag::PieceContent);
    assert_eq!(&response.to_bytes()[6..], &piece);
}

#[test]
fn test_error_propagation() {
    let peer = MockPeer::new();
    let task_id = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

    // Test with missing piece
    let request = Packet::new(
        Tag::DownloadPiece,
        format!("{}-42", task_id).into_bytes().into(),
    )
    .unwrap();
    let result = peer.handle_packet(&request);
    assert!(matches!(result, Err(Error::InvalidPacket(_))));

    // Test with unexpected tag
    let request = Packet::new(Tag::PieceContent, vec![1, 2, 3, 4].into()).unwrap();
    let result = peer.handle_packet(&request);
    assert!(matches!(result, Err(Error::InvalidPacket(_))));
}

#[test]
fn test_invalid_length() {
    // Create a packet with invalid length in the header
    let mut packet_bytes = BytesMut::with_capacity(6);
    packet_bytes.put_u8(42); // packet_id
    packet_bytes.put_u8(Tag::PieceContent.into()); // tag
    packet_bytes.put_u32(u32::MAX); // length (too large)

    // Attempt to parse the packet with invalid length
    let result = Packet::from_bytes(packet_bytes.freeze());
    assert!(matches!(result, Err(Error::InvalidLength(_))));
}

#[test]
fn test_invalid_format() {
    // Test with invalid format (missing separator)
    let task_id = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let result = Packet::new(Tag::DownloadPiece, task_id.as_bytes().to_vec().into());
    assert!(matches!(result, Err(Error::InvalidPacket(_))));

    // Test with invalid piece ID format
    let result = Packet::new(
        Tag::DownloadPiece,
        format!("{}-abc", task_id).into_bytes().into(),
    );
    assert!(matches!(result, Err(Error::ParseIntError(_))));
}
