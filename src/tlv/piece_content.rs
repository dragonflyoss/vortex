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
use bytes::Bytes;

/// MAX_PIECE_SIZE is the maximum size of a piece content (4 GiB).
const MAX_PIECE_SIZE: usize = crate::MAX_VALUE_SIZE;

/// PieceContent represents the content of a piece.
#[derive(Debug, Clone)]
pub struct PieceContent(Bytes);

/// PieceContent implements the PieceContent functions.
impl PieceContent {
    /// new creates a new piece content.
    pub fn new(content: Bytes) -> Result<Self> {
        // Check content length
        if content.len() > MAX_PIECE_SIZE {
            return Err(Error::InvalidLength(format!(
                "content length {} exceeds maximum size {}",
                content.len(),
                MAX_PIECE_SIZE
            )));
        }

        Ok(PieceContent(content))
    }

    /// len returns the length of the piece content.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// is_empty returns whether the piece content is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// from_bytes creates a piece content from a byte slice.
    pub fn from_bytes(bytes: Bytes) -> Result<Self> {
        Self::new(bytes)
    }

    /// to_bytes converts the piece content to a byte slice.
    pub fn to_bytes(&self) -> Bytes {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let content = vec![1, 2, 3, 4];
        let result = PieceContent::new(content.into());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 4);
    }

    #[test]
    fn test_is_empty() {
        let content = Bytes::new();
        let result = PieceContent::new(content);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_to_bytes_and_from_bytes() {
        let content = vec![1, 2, 3, 4];
        let piece_content = PieceContent::new(content.clone().into()).unwrap();
        let bytes = piece_content.to_bytes();
        let result = PieceContent::from_bytes(bytes);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_bytes(), content);
    }

    #[test]
    fn test_from_bytes_invalid_input() {
        // Test empty input
        let result = PieceContent::from_bytes(Bytes::new());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        // Test oversize input
        let large_content = vec![0; MAX_PIECE_SIZE + 1];
        let result: std::result::Result<PieceContent, Error> =
            PieceContent::from_bytes(large_content.into());
        assert!(matches!(result, Err(Error::InvalidLength(_))));
    }
}
