//! Value layouts and scalar aliases shared by the supported game ABI.

/// A four-byte SDK hash value.
///
/// This is an owned scalar, not the SDK's C++ `CHashString` wrapper. Converting
/// text to a hash requires calling the game's build-specific `HashString`
/// function and is intentionally kept out of this type.
#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HashId(u32);

impl HashId {
    /// The SDK's empty hash value.
    pub const EMPTY: Self = Self(0);

    /// Convert the SDK constructor's raw integer input to its stored value.
    ///
    /// The community SDK treats `0xDEADBEEF` as an uninitialized sentinel and
    /// stores zero instead. Other values are preserved verbatim.
    pub const fn from_sdk_raw(raw: u32) -> Self {
        if raw == 0xDEAD_BEEF {
            Self::EMPTY
        } else {
            Self(raw)
        }
    }

    /// Return the four-byte value stored by the game.
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// The SDK's pointer-free `CVector2f` value.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Vector2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// The SDK's pointer-free `CVector3f` value.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn magnitude(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn distance(self, other: Self) -> f32 {
        (self - other).magnitude()
    }

    /// Component-wise interpolation, matching the SDK's `Lerp` helper.
    pub fn lerp(from: Self, to: Self, progress: f32) -> Self {
        from * (1.0 - progress) + to * progress
    }
}

impl core::ops::Add for Vector3 {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl core::ops::Sub for Vector3 {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl core::ops::Mul for Vector3 {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        Self::new(self.x * other.x, self.y * other.y, self.z * other.z)
    }
}

impl core::ops::Mul<f32> for Vector3 {
    type Output = Self;

    fn mul(self, scale: f32) -> Self::Output {
        Self::new(self.x * scale, self.y * scale, self.z * scale)
    }
}

/// The SDK's four-float vector value. `from_vector3` supplies a homogeneous w of 1.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vector4 {
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub const fn from_vector3(value: Vector3) -> Self {
        Self::new(value.x, value.y, value.z, 1.0)
    }
}

/// The SDK's plane value (`Normal` followed by scalar `D`).
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Plane {
    pub normal: Vector3,
    pub d: f32,
}

/// The SDK's axis-aligned box value, represented as two copied vectors.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct AxisAlignedBox {
    pub min: Vector3,
    pub max: Vector3,
}

/// The SDK's pointer-free `CQuaternion` storage.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quaternion {
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn set(&mut self, x: f32, y: f32, z: f32, w: f32) {
        *self = Self::new(x, y, z, w);
    }
}

/// A row-major 4×4 float matrix matching the SDK's `CMatrix4f` storage.
///
/// This value type does not by itself establish the engine's coordinate
/// convention. The current project reads translation from row 3, which is
/// reflected by [`Self::translation`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix4x4 {
    pub values: [[f32; 4]; 4],
}

impl Matrix4x4 {
    pub const fn identity() -> Self {
        Self::from_rows([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub const fn from_rows(rows: [[f32; 4]; 4]) -> Self {
        Self { values: rows }
    }

    pub const fn right(self) -> Vector3 {
        Vector3::new(self.values[0][0], self.values[0][1], self.values[0][2])
    }

    pub const fn forward(self) -> Vector3 {
        Vector3::new(self.values[1][0], self.values[1][1], self.values[1][2])
    }

    pub const fn up(self) -> Vector3 {
        Vector3::new(self.values[2][0], self.values[2][1], self.values[2][2])
    }

    pub const fn translation(self) -> Vector3 {
        Vector3::new(self.values[3][0], self.values[3][1], self.values[3][2])
    }

    pub fn set_right(&mut self, value: Vector3) {
        self.values[0] = [value.x, value.y, value.z, 0.0];
    }

    pub fn set_forward(&mut self, value: Vector3) {
        self.values[1] = [value.x, value.y, value.z, 0.0];
    }

    pub fn set_up(&mut self, value: Vector3) {
        self.values[2] = [value.x, value.y, value.z, 0.0];
    }

    pub fn set_translation(&mut self, value: Vector3) {
        self.values[3] = [value.x, value.y, value.z, 1.0];
    }
}

impl Default for Matrix4x4 {
    fn default() -> Self {
        Self::identity()
    }
}

impl core::ops::Mul for Matrix4x4 {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        let mut product = [[0.0; 4]; 4];
        for (row_index, row) in product.iter_mut().enumerate() {
            for (column_index, value) in row.iter_mut().enumerate() {
                *value = self.values[row_index][0] * other.values[0][column_index]
                    + self.values[row_index][1] * other.values[1][column_index]
                    + self.values[row_index][2] * other.values[2][column_index]
                    + self.values[row_index][3] * other.values[3][column_index];
            }
        }
        Self::from_rows(product)
    }
}

#[cfg(test)]
mod tests {
    use super::{AxisAlignedBox, HashId, Matrix4x4, Plane, Quaternion, Vector2, Vector3, Vector4};

    #[test]
    fn hash_id_maps_sdk_uninitialized_sentinel_to_zero() {
        assert_eq!(HashId::from_sdk_raw(0xDEAD_BEEF), HashId::EMPTY);
        assert_eq!(HashId::from_sdk_raw(0x1234_5678).raw(), 0x1234_5678);
    }

    #[test]
    fn vector3_magnitude_matches_three_component_euclidean_length() {
        assert_eq!(Vector3::new(3.0, 4.0, 12.0).magnitude(), 13.0);
    }

    #[test]
    fn matrix4x4_has_native_float_matrix_size_and_alignment() {
        assert_eq!(std::mem::size_of::<Matrix4x4>(), 64);
        assert_eq!(std::mem::align_of::<Matrix4x4>(), 4);
    }

    #[test]
    fn matrix4x4_reads_translation_from_fourth_row() {
        let matrix = Matrix4x4::from_rows([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [12.0, -3.0, 7.5, 1.0],
        ]);
        assert_eq!(matrix.translation(), Vector3::new(12.0, -3.0, 7.5));
    }

    #[test]
    fn vector3_distance_and_lerp_match_the_sdk_math_helpers() {
        let from = Vector3::new(0.0, 2.0, -2.0);
        let to = Vector3::new(3.0, 6.0, 1.0);
        assert!((from.distance(to) - 34.0_f32.sqrt()).abs() < f32::EPSILON);
        assert_eq!(Vector3::lerp(from, to, 0.5), Vector3::new(1.5, 4.0, -0.5));
    }

    #[test]
    fn matrix_basis_and_position_setters_use_sdk_row_layout() {
        let mut matrix = Matrix4x4::identity();
        matrix.set_right(Vector3::new(0.0, 1.0, 0.0));
        matrix.set_forward(Vector3::new(1.0, 0.0, 0.0));
        matrix.set_up(Vector3::new(0.0, 0.0, -1.0));
        matrix.set_translation(Vector3::new(4.0, 5.0, 6.0));

        assert_eq!(matrix.right(), Vector3::new(0.0, 1.0, 0.0));
        assert_eq!(matrix.forward(), Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(matrix.up(), Vector3::new(0.0, 0.0, -1.0));
        assert_eq!(matrix.translation(), Vector3::new(4.0, 5.0, 6.0));
        assert_eq!(matrix.values[3][3], 1.0);
    }

    #[test]
    fn matrix4x4_multiplication_uses_row_major_product() {
        let left = Matrix4x4::from_rows([
            [1.0, 2.0, 3.0, 4.0],
            [0.0, 1.0, 2.0, 3.0],
            [0.0, 0.0, 1.0, 2.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        assert_eq!((left * Matrix4x4::identity()).values, left.values);
        assert_eq!(
            (left * left).values,
            [
                [1.0, 4.0, 10.0, 20.0],
                [0.0, 1.0, 4.0, 10.0],
                [0.0, 0.0, 1.0, 4.0],
                [0.0, 0.0, 0.0, 1.0],
            ]
        );
    }

    #[test]
    fn sdk_math_value_layouts_match_float_storage() {
        assert_eq!(std::mem::size_of::<HashId>(), 4);
        assert_eq!(std::mem::size_of::<Vector2>(), 8);
        assert_eq!(std::mem::size_of::<Vector3>(), 12);
        assert_eq!(std::mem::size_of::<Vector4>(), 16);
        assert_eq!(std::mem::size_of::<Plane>(), 16);
        assert_eq!(std::mem::size_of::<AxisAlignedBox>(), 24);
        assert_eq!(std::mem::size_of::<Quaternion>(), 16);
    }

    #[test]
    fn quaternion_set_replaces_all_four_components() {
        let mut quaternion = Quaternion::default();
        quaternion.set(1.0, 2.0, 3.0, 4.0);
        assert_eq!(quaternion, Quaternion::new(1.0, 2.0, 3.0, 4.0));
    }
}
