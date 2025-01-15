/*
 *     Copyright 2025 The Dragonfly Authors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::error::{Error, Result};
use bytes::{BufMut, Bytes, BytesMut};
use rand::prelude::*;

pub mod error;
pub mod tlv;

/// HEADER_SIZE is the size of the Vortex packet header including the packet identifier, tag, and
/// length.
const HEADER_SIZE: usize = 6;

/// MAX_VALUE_SIZE is the maximum size of the value field (4 GiB).
const MAX_VALUE_SIZE: usize = 4 * 1024 * 1024 * 1024;

/// Header represents the Vortex packet header.
#[derive(Debug)]
pub struct Header {
    packet_id: u8,
    tag: tlv::Tag,
    length: usize,
}

/// Vortex Protocol
///
/// Vortex is a peer-to-peer (P2P) file transfer protocol using TLV (Tag-Length-Value) format for
/// efficient and flexible data transmission. Designed for reliable and scalable file sharing.
///
/// Packet Format:
///     - Packet Identifier (8 bits): Uniquely identifies each packet
///     - Tag (8 bits): Specifies data type in value field
///     - Length (32 bits): Indicates Value field length, up to 4 GiB
///     - Value (variable): Actual data content, maximum 1 GiB
///
/// Protocol Format:
///
/// ```text
/// -------------------------------------------------------------------------------------------------
/// |                            |                   |                    |                         |
/// | Packet Identifier (8 bits) |    Tag (8 bits)   |  Length (32 bits)  |   Value (up to 4 GiB)   |
/// |                            |                   |                    |                         |
/// -------------------------------------------------------------------------------------------------
/// ```
///
/// For more information, please refer to the [Vortex Protocol](https://github.com/dragonflyoss/vortex/blob/main/docs/README.md).
#[derive(Debug)]
pub enum Vortex {
    DownloadPiece(Header, tlv::download_piece::DownloadPiece),
    PieceContent(Header, tlv::piece_content::PieceContent),
    Reserved(Header),
    Error(Header, tlv::error::Error),
}

/// Vortex implements the Vortex functions.
impl Vortex {
    /// Creates a new Vortex packet.
    pub fn new(tag: tlv::Tag, value: Bytes) -> Result<Self> {
        // Check value length
        if value.len() > MAX_VALUE_SIZE {
            return Err(Error::InvalidLength(format!(
                "value length {} exceeds maximum size {}",
                value.len(),
                MAX_VALUE_SIZE
            )));
        }

        let mut rng = thread_rng();
        let header = Header {
            packet_id: rng.gen(),
            tag,
            length: value.len(),
        };

        match tag {
            tlv::Tag::DownloadPiece => {
                let download_piece = tlv::download_piece::DownloadPiece::from_bytes(value)?;
                Ok(Vortex::DownloadPiece(header, download_piece))
            }
            tlv::Tag::PieceContent => {
                let piece_content = tlv::piece_content::PieceContent::from_bytes(value)?;
                Ok(Vortex::PieceContent(header, piece_content))
            }
            tlv::Tag::Reserved(_) => Ok(Vortex::Reserved(header)),
            tlv::Tag::Error => {
                let err = tlv::error::Error::from_bytes(value)?;
                Ok(Vortex::Error(header, err))
            }
        }
    }

    /// packet_id returns the packet identifier of the Vortex packet.
    #[inline]
    pub fn packet_id(&self) -> u8 {
        match self {
            Vortex::DownloadPiece(header, _) => header.packet_id,
            Vortex::PieceContent(header, _) => header.packet_id,
            Vortex::Reserved(header) => header.packet_id,
            Vortex::Error(header, _) => header.packet_id,
        }
    }

    /// tag returns the tag of the Vortex packet.
    #[inline]
    pub fn tag(&self) -> &tlv::Tag {
        match self {
            Vortex::DownloadPiece(header, _) => &header.tag,
            Vortex::PieceContent(header, _) => &header.tag,
            Vortex::Reserved(header) => &header.tag,
            Vortex::Error(header, _) => &header.tag,
        }
    }

    /// length returns the length of the value field.
    #[inline]
    pub fn length(&self) -> usize {
        match self {
            Vortex::DownloadPiece(header, _) => header.length,
            Vortex::PieceContent(header, _) => header.length,
            Vortex::Reserved(header) => header.length,
            Vortex::Error(header, _) => header.length,
        }
    }

    /// from_bytes creates a Vortex packet from a byte slice.
    pub fn from_bytes(bytes: Bytes) -> Result<Self> {
        if bytes.len() < HEADER_SIZE {
            return Err(Error::InvalidPacket(format!(
                "expected min {HEADER_SIZE} bytes, got {}",
                bytes.len()
            )));
        }

        let mut bytes = BytesMut::from(bytes);
        let header = bytes.split_to(HEADER_SIZE);
        let value = bytes;
        let packet_id = header[0];
        let tag = header[1]
            .try_into()
            .map_err(|err| Error::InvalidPacket(format!("invalid tag value: {:?}", err)))?;
        let length = u32::from_be_bytes(header[2..HEADER_SIZE].try_into()?) as usize;

        // Check if the value length matches the specified length.
        if value.len() != length {
            return Err(Error::InvalidLength(format!(
                "value len {} != declared length {}",
                value.len(),
                length
            )));
        }

        // Check if the value length exceeds the maximum size.
        if length > MAX_VALUE_SIZE {
            return Err(Error::InvalidLength(format!(
                "value length {} exceeds maximum size {}",
                length, MAX_VALUE_SIZE
            )));
        }

        let header = Header {
            packet_id,
            tag,
            length,
        };

        match tag {
            tlv::Tag::DownloadPiece => {
                let download_piece =
                    tlv::download_piece::DownloadPiece::from_bytes(value.freeze())?;
                Ok(Vortex::DownloadPiece(header, download_piece))
            }
            tlv::Tag::PieceContent => {
                let piece_content = tlv::piece_content::PieceContent::from_bytes(value.freeze())?;
                Ok(Vortex::PieceContent(header, piece_content))
            }
            tlv::Tag::Reserved(_) => Ok(Vortex::Reserved(header)),
            tlv::Tag::Error => {
                let error = tlv::error::Error::from_bytes(value.freeze())?;
                Ok(Vortex::Error(header, error))
            }
        }
    }

    /// to_bytes converts the Vortex packet to a byte slice.
    pub fn to_bytes(&self) -> Bytes {
        let (header, value) = match self {
            Vortex::DownloadPiece(header, download_piece) => (header, download_piece.to_bytes()),
            Vortex::PieceContent(header, piece_content) => (header, piece_content.to_bytes()),
            Vortex::Reserved(header) => (header, Bytes::new()),
            Vortex::Error(header, err) => (header, err.to_bytes()),
        };

        let mut bytes = BytesMut::with_capacity(HEADER_SIZE + value.len());
        bytes.put_u8(header.packet_id);
        bytes.put_u8(header.tag.into());
        bytes.put_u32(value.len() as u32);
        bytes.extend_from_slice(&value);
        bytes.freeze()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tlv::Tag;
    use bytes::Bytes;

    #[test]
    fn test_new_download_piece() {
        let tag = Tag::DownloadPiece;
        let value = Bytes::from("a".repeat(32) + "-42");
        let packet = Vortex::new(tag, value.clone()).expect("Failed to create Vortex packet");

        assert_eq!(packet.packet_id(), packet.packet_id());
        assert_eq!(packet.tag(), &tag);
        assert_eq!(packet.length(), value.len());
    }

    #[test]
    fn test_new_piece_content() {
        let value = b"piece content";
        let packet = Vortex::new(Tag::PieceContent, Bytes::from_static(value))
            .expect("Failed to create packet");
        assert_eq!(packet.tag(), &Tag::PieceContent);
        assert_eq!(packet.length(), value.len());
    }

    #[test]
    fn test_from_bytes() {
        let value = b"test data";
        let packet = Vortex::new(Tag::PieceContent, Bytes::from_static(value))
            .expect("Failed to create packet");
        let bytes = packet.to_bytes();
        let deserialized = Vortex::from_bytes(bytes).expect("Failed to deserialize packet");
        assert_eq!(packet.tag(), deserialized.tag());
        assert_eq!(packet.length(), deserialized.length());
    }

    #[test]
    fn test_to_bytes() {
        let value = b"test data";
        let packet = Vortex::new(Tag::PieceContent, Bytes::from_static(value))
            .expect("Failed to create packet");
        let bytes = packet.to_bytes();
        assert_eq!(bytes.len(), HEADER_SIZE + value.len());
    }

    #[test]
    fn test_error_handling() {
        // Test invalid length
        let value = vec![0; MAX_VALUE_SIZE + 1];
        let result = Vortex::new(Tag::PieceContent, value.into());
        assert!(matches!(result, Err(Error::InvalidLength(_))));
    }
}
