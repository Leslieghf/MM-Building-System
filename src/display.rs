//! Shared display formatting for native metrics and script output.

const FIXED_DECIMAL_PLACES: u32 = 3;
const FIXED_SCALE: f64 = 1000.0;

/// Truncate toward zero and always render three fractional digits.
pub(crate) fn fixed(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }

    let magnitude = value.abs();
    let whole = magnitude.trunc();
    let fraction = ((magnitude - whole) * FIXED_SCALE).trunc() as u16;
    let sign = if value.is_sign_negative() && (whole != 0.0 || fraction != 0) {
        "-"
    } else {
        ""
    };
    format!(
        "{sign}{whole:.0}.{fraction:0width$}",
        width = FIXED_DECIMAL_PLACES as usize
    )
}

pub(crate) fn vector(value: Option<[f32; 3]>) -> String {
    value.map_or_else(
        || "--".to_owned(),
        |components| vector_components(components.map(f64::from)),
    )
}

pub(crate) fn vector_components(components: [f64; 3]) -> String {
    components
        .into_iter()
        .map(fixed)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn angles(value: Option<[f32; 3]>) -> String {
    value.map_or_else(
        || "--".to_owned(),
        |components| {
            components
                .into_iter()
                .map(|component| format!("{}°", fixed(f64::from(component))))
                .collect::<Vec<_>>()
                .join(", ")
        },
    )
}

pub(crate) fn magnitude(value: Option<[f32; 3]>, suffix: &str) -> String {
    value.map_or_else(
        || "--".to_owned(),
        |[x, y, z]| {
            let magnitude = ((x as f64).powi(2) + (y as f64).powi(2) + (z as f64).powi(2)).sqrt();
            format!("{}{suffix}", fixed(magnitude))
        },
    )
}
