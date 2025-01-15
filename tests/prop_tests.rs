use bytes::{BufMut, Bytes, BytesMut};
use proptest::prelude::*;
use proptest::test_runner::TestRunner;
use vortex::error::Error;
use vortex::tlv::Tag;
use vortex::Vortex as Packet;

// Generate valid value bytes based on tag
fn generate_value_bytes(tag: Tag) -> Bytes {
    match tag {
        Tag::DownloadPiece => {
            // Task ID must be 32 bytes (64 hex chars)
            let task_id = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
            let piece_id = 42;
            format!("{}-{}", task_id, piece_id).into_bytes().into()
        }
        Tag::PieceContent => {
            // PieceContent can be any bytes
            vec![1, 2, 3, 4].into()
        }
        Tag::Error => {
            // Error format is "code:message"
            format!("{}:test error", 1u8).into_bytes().into()
        }
        Tag::Reserved(_) => {
            // Reserved tags can have any format
            vec![].into()
        }
    }
}

// Generate arbitrary valid packets
fn arb_packet() -> impl Strategy<Value = Packet> {
    let arb_tag = prop_oneof![
        Just(Tag::DownloadPiece),
        Just(Tag::PieceContent),
        Just(Tag::Error)
    ];

    arb_tag.prop_map(|tag| {
        let value = generate_value_bytes(tag);
        Packet::new(tag, value).expect("Failed to create packet")
    })
}

// Run tests with custom configuration to minimize output
fn run_test<F>(test: F)
where
    F: Fn(Packet) -> Result<(), TestCaseError>,
{
    let config = ProptestConfig {
        max_shrink_iters: 0,       // Disable shrinking to reduce output
        cases: 100,                // Reduce number of test cases
        failure_persistence: None, // Don't persist failures
        ..Default::default()
    };
    let mut runner = TestRunner::new(config);
    runner.run(&arb_packet(), test).unwrap();
}

#[test]
fn test_roundtrip_serialization() {
    run_test(|packet| {
        let serialized = packet.to_bytes();
        let deserialized = Packet::from_bytes(serialized).expect("Failed to deserialize packet");

        prop_assert_eq!(packet.packet_id(), deserialized.packet_id());
        prop_assert_eq!(packet.tag(), deserialized.tag());
        prop_assert_eq!(packet.length(), deserialized.length());
        Ok(())
    });
}

#[test]
fn test_packet_length() {
    run_test(|packet| {
        let bytes = packet.to_bytes();
        prop_assert_eq!(bytes.len(), packet.length() + 6); // header size is 6
        Ok(())
    });
}

// Test that invalid lengths are rejected
#[test]
fn test_invalid_length() {
    let tag = Tag::PieceContent;

    // Create a large value that will exceed the maximum size
    let mut bytes = BytesMut::with_capacity(8);
    bytes.extend_from_slice(&[1, 2, 3, 4]);

    // Create a packet with invalid length in the header
    let mut packet_bytes = BytesMut::with_capacity(6 + bytes.len());
    packet_bytes.put_u8(42); // packet_id
    packet_bytes.put_u8(tag.into()); // tag
    packet_bytes.put_u32(u32::MAX); // length (too large)
    packet_bytes.extend_from_slice(&bytes);

    // Attempt to parse the packet with invalid length
    let result = Packet::from_bytes(packet_bytes.freeze());
    assert!(matches!(result, Err(Error::InvalidLength(_))));
}

// Test that packet values match expectations
#[test]
fn test_packet_value_constraints() {
    run_test(|packet| {
        let bytes = packet.to_bytes();
        prop_assert!(bytes.len() >= 6);
        prop_assert!(matches!(
            packet.tag(),
            &Tag::DownloadPiece | &Tag::PieceContent | &Tag::Error
        ));
        Ok(())
    });
}

// Integration-style property tests
#[test]
fn test_download_piece_format() {
    // Task ID must be 32 bytes (64 hex chars)
    let task_id = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let piece_id = 42;
    let value = format!("{}-{}", task_id, piece_id);

    let packet = Packet::new(Tag::DownloadPiece, value.into_bytes().into())
        .expect("Failed to create download piece packet");

    assert_eq!(packet.tag(), &Tag::DownloadPiece);

    let value_str = String::from_utf8(packet.to_bytes()[6..].to_vec())
        .expect("Failed to convert value to string");
    let parts: Vec<&str> = value_str.split('-').collect();

    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], task_id);
    assert_eq!(parts[1], piece_id.to_string());
}
