//! Candidate string-hashing call shape from the pinned `core/hash.h`.
//!
//! The returned 32-bit hash is not equivalent to [`super::super::types::HashId`]
//! construction for arbitrary text unless the game's function is called. This
//! module contains no mapped address or safe Rust hashing substitute.
