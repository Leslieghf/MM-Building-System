//! Opaque markers for the SDK's multiple-inheritance game-object hierarchy.
//!
//! `pfxgo.h`, `physgo.h`, and `damageable.h` declare virtual methods and
//! engine-owned state. These markers prevent accidental type confusion, but do
//! not model MSVC subobject offsets or authorize virtual calls.

/// Opaque `IPfxGameObject` base subobject.
pub enum PfxGameObject {}

/// Opaque `CPhysicsGameObject` base subobject.
pub enum PhysicsGameObject {}

/// Opaque `CDamageable` base subobject.
pub enum Damageable {}
