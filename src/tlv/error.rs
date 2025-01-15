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

use crate::error::{Error as VortexError, Result as VortexResult};
use bytes::{BufMut, Bytes, BytesMut};

/// SEPARATOR is the separator character used in the error request, separating the error code
/// and error message. It is a hyphen character ':'.
const SEPARATOR: u8 = b':';

/// Code represents a error code.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// Unknown error.
    Unknown = 0,

    /// Invalid argument, such as an invalid task ID or piece number.
    InvalidArgument = 1,

    /// Resource not found, such as a missing piece.
    NotFound = 2,

    /// Internal error, such as an unexpected error.
    Internal = 3,

    /// Reserved for future use.
    Reserved(u8), // For tags 4-254
}

/// Implement TryFrom<u8> for Code.
impl TryFrom<u8> for Code {
    type Error = ();

    /// Converts a u8 to a Code enum.
    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(Code::Unknown),
            1 => Ok(Code::InvalidArgument),
            2 => Ok(Code::NotFound),
            3 => Ok(Code::Internal),
            4..=255 => Ok(Code::Reserved(value)),
        }
    }
}

/// Implement From<Code> for u8.
impl From<Code> for u8 {
    /// Converts a Code enum to a u8.
    fn from(code: Code) -> u8 {
        match code {
            Code::Unknown => 0,
            Code::InvalidArgument => 1,
            Code::NotFound => 2,
            Code::Internal => 3,
            Code::Reserved(value) => value,
        }
    }
}

/// Error represents a error request.
///
/// Value Format:
///  - Error Code (8 bits): Error code.
///  - Separator (1 byte): Separator character ':'.
///  - Error Message (variable): Error message.
///
/// ```text
/// ----------------------------------------------------------------------
/// | Error Code (8 bits) | Separator (1 byte) |      Error Message      |
/// ----------------------------------------------------------------------
/// ```
#[derive(Debug, Clone)]
pub struct Error {
    code: Code,
    message: String,
}

/// Error implements the Error functions.
impl Error {
    /// new creates a new Error request.
    pub fn new(code: Code, message: String) -> Self {
        Error { code, message }
    }

    /// code returns the error code.
    pub fn code(&self) -> Code {
        self.code
    }

    /// message returns the error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// len returns the length of the error request, including the error code, separator, and error
    /// message.
    pub fn len(&self) -> usize {
        self.message.len() + 2
    }

    /// is_empty returns whether the error request is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// from_bytes creates a error request from a byte slice.
    pub fn from_bytes(bytes: Bytes) -> VortexResult<Self> {
        let mut parts = bytes.splitn(2, |&b| b == SEPARATOR);

        let code = parts
            .next()
            .ok_or(VortexError::InvalidPacket("missing error code".to_string()))?;

        if code.len() != 1 {
            return Err(VortexError::InvalidPacket("invalid error code".to_string()));
        }

        let code: Code = code
            .first()
            .copied()
            .ok_or(VortexError::InvalidPacket("missing error code".to_string()))?
            .try_into()
            .unwrap();

        let message = std::str::from_utf8(parts.next().ok_or(VortexError::InvalidPacket(
            "missing error message".to_string(),
        ))?)?
        .to_string();

        if message.is_empty() {
            return Err(VortexError::InvalidPacket(
                "empty error message".to_string(),
            ));
        }

        Ok(Error { code, message })
    }

    /// to_bytes converts the error request to a byte slice.
    pub fn to_bytes(&self) -> Bytes {
        let mut bytes = BytesMut::with_capacity(self.len());
        bytes.put_u8(self.code.into());
        bytes.put_u8(SEPARATOR);
        bytes.extend_from_slice(self.message.as_bytes());
        bytes.freeze()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_new() {
        let code = Code::InvalidArgument;
        let message = "Invalid argument".to_string();
        let error = Error::new(code, message.clone());

        assert_eq!(error.code(), code);
        assert_eq!(error.message(), message);
        assert_eq!(error.len(), message.len() + 2);
    }

    #[test]
    fn test_is_non_empty() {
        let error_non_empty = Error::new(Code::Unknown, "Error message".to_string());
        assert!(!error_non_empty.is_empty());
    }

    #[test]
    fn test_to_bytes_and_from_bytes() {
        let code = Code::NotFound;
        let message = "Resource not found".to_string();
        let error = Error::new(code, message.clone());

        let bytes = error.to_bytes();
        let error_decoded = Error::from_bytes(bytes).unwrap();

        assert_eq!(error_decoded.code(), code);
        assert_eq!(error_decoded.message(), message);
    }

    #[test]
    fn test_from_bytes_invalid_input() {
        // Test missing separator
        let invalid_bytes = Bytes::from("invalid_input_without_separator");
        assert!(Error::from_bytes(invalid_bytes).is_err());

        // Test missing error message
        let invalid_bytes = Bytes::from(format!("{}:", 1));
        assert!(Error::from_bytes(invalid_bytes).is_err());

        // Test invalid error code format
        let invalid_bytes = Bytes::from(format!("{}:{}", 256, "Invalid code"));
        assert!(Error::from_bytes(invalid_bytes).is_err());
    }
}
