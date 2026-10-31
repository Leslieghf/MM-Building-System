//! Diagnostic types for observed native values; these identities are not object handles.
/// Diagnostic identity captured from the accepted shape pointer in a native
/// world-query context. These addresses are observations, not safe object handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Observed identity of a candidate shape; addresses are diagnostic only.
pub struct RaycastCandidateShape {
    /// Native address observed for the candidate shape.
    pub address: usize,
    /// Native vtable address observed for the shape.
    pub vtable: usize,
}
