//! Pure, finite-checked vector math exposed to Rhai scripts.
//!
//! # Semantics
//!
//! Inputs must contain finite components. Operations return a Rhai error for
//! invalid values; normalization and direction return `()` when the vector is
//! too small to produce a stable unit vector. These helpers do not read game
//! state and require no capability. Their script-visible names and signatures
//! are listed in [`super::catalog`].
use rhai::{Dynamic, Engine, FLOAT, Map};

const COMPONENT_LIMIT: FLOAT = 100_000.0;
const ZERO_EPSILON: FLOAT = 1e-12;

type Vector = [FLOAT; 3];
type MathResult<T> = Result<T, Box<rhai::EvalAltResult>>;

#[derive(Clone, Copy, Default)]
pub(crate) struct ScriptMath;

impl ScriptMath {
    pub(crate) fn register(engine: &mut Engine) {
        engine
            .register_type_with_name::<Self>("MadMaxScriptMath")
            .register_fn("vec3", |_math: &mut Self, x, y, z| Self::vec3(x, y, z))
            .register_fn("add", |_math: &mut Self, a, b| Self::add(a, b))
            .register_fn("subtract", |_math: &mut Self, a, b| Self::subtract(a, b))
            .register_fn("scale", |_math: &mut Self, v, scalar| {
                Self::scale(v, scalar)
            })
            .register_fn("dot", |_math: &mut Self, a, b| Self::dot(a, b))
            .register_fn("cross", |_math: &mut Self, a, b| Self::cross(a, b))
            .register_fn("length", |_math: &mut Self, v| Self::length(v))
            .register_fn("distance", |_math: &mut Self, a, b| Self::distance(a, b))
            .register_fn("normalize", |_math: &mut Self, v| {
                Self::normalize(v).map(option_to_dynamic)
            })
            .register_fn("direction", |_math: &mut Self, from, to| {
                Self::direction(from, to).map(option_to_dynamic)
            })
            .register_fn("lerp", |_math: &mut Self, a, b, t| Self::lerp(a, b, t));
    }

    pub(crate) fn vec3(x: FLOAT, y: FLOAT, z: FLOAT) -> MathResult<Map> {
        Self::vector([x, y, z])
    }

    pub(crate) fn add(a: Map, b: Map) -> MathResult<Map> {
        let a = parse_vector(&a)?;
        let b = parse_vector(&b)?;
        Self::vector([a[0] + b[0], a[1] + b[1], a[2] + b[2]])
    }

    pub(crate) fn subtract(a: Map, b: Map) -> MathResult<Map> {
        let a = parse_vector(&a)?;
        let b = parse_vector(&b)?;
        Self::vector([a[0] - b[0], a[1] - b[1], a[2] - b[2]])
    }

    pub(crate) fn scale(v: Map, scalar: FLOAT) -> MathResult<Map> {
        let v = parse_vector(&v)?;
        validate_scalar(scalar, "scalar")?;
        Self::vector([v[0] * scalar, v[1] * scalar, v[2] * scalar])
    }

    pub(crate) fn dot(a: Map, b: Map) -> MathResult<FLOAT> {
        let a = parse_vector(&a)?;
        let b = parse_vector(&b)?;
        finite_result(a[0] * b[0] + a[1] * b[1] + a[2] * b[2], "dot product")
    }

    pub(crate) fn cross(a: Map, b: Map) -> MathResult<Map> {
        let a = parse_vector(&a)?;
        let b = parse_vector(&b)?;
        Self::vector([
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ])
    }

    pub(crate) fn length(v: Map) -> MathResult<FLOAT> {
        let v = parse_vector(&v)?;
        finite_result(v[0].hypot(v[1]).hypot(v[2]), "length")
    }

    pub(crate) fn distance(a: Map, b: Map) -> MathResult<FLOAT> {
        let a = parse_vector(&a)?;
        let b = parse_vector(&b)?;
        finite_result(
            (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]),
            "distance",
        )
    }

    pub(crate) fn normalize(v: Map) -> MathResult<Option<Map>> {
        let v = parse_vector(&v)?;
        let magnitude = v[0].hypot(v[1]).hypot(v[2]);
        if magnitude <= ZERO_EPSILON {
            return Ok(None);
        }
        Self::vector([v[0] / magnitude, v[1] / magnitude, v[2] / magnitude]).map(Some)
    }

    pub(crate) fn direction(from: Map, to: Map) -> MathResult<Option<Map>> {
        let from = parse_vector(&from)?;
        let to = parse_vector(&to)?;
        let delta = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        let magnitude = delta[0].hypot(delta[1]).hypot(delta[2]);
        if magnitude <= ZERO_EPSILON {
            return Ok(None);
        }
        Self::vector([
            delta[0] / magnitude,
            delta[1] / magnitude,
            delta[2] / magnitude,
        ])
        .map(Some)
    }

    pub(crate) fn lerp(a: Map, b: Map, t: FLOAT) -> MathResult<Map> {
        let a = parse_vector(&a)?;
        let b = parse_vector(&b)?;
        if !t.is_finite() || !(0.0..=1.0).contains(&t) {
            return Err("lerp parameter must be finite and within [0, 1]".into());
        }
        Self::vector([
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
        ])
    }

    fn vector(values: Vector) -> MathResult<Map> {
        for value in values {
            validate_component(value)?;
        }
        Ok(["x", "y", "z"]
            .into_iter()
            .zip(values)
            .map(|(key, value)| (key.into(), Dynamic::from(value)))
            .collect())
    }
}

fn option_to_dynamic(value: Option<Map>) -> Dynamic {
    value.map(Dynamic::from).unwrap_or(Dynamic::UNIT)
}

fn parse_vector(map: &Map) -> MathResult<Vector> {
    if map.len() != 3 || !["x", "y", "z"].iter().all(|key| map.contains_key(*key)) {
        return Err("vector must contain exactly numeric x, y, and z components".into());
    }
    let mut values = [0.0; 3];
    for (index, key) in ["x", "y", "z"].iter().enumerate() {
        let component = &map[*key];
        let value = component
            .clone()
            .try_cast::<FLOAT>()
            .or_else(|| {
                component
                    .clone()
                    .try_cast::<rhai::INT>()
                    .map(|value| value as FLOAT)
            })
            .ok_or_else(|| Box::<rhai::EvalAltResult>::from("vector components must be numbers"))?;
        validate_component(value)?;
        values[index] = value;
    }
    Ok(values)
}

fn validate_component(value: FLOAT) -> MathResult<()> {
    if !value.is_finite() || value.abs() > COMPONENT_LIMIT {
        return Err("vector components must be finite and within [-100000, 100000]".into());
    }
    Ok(())
}

fn validate_scalar(value: FLOAT, name: &str) -> MathResult<()> {
    if !value.is_finite() {
        return Err(format!("{name} must be finite").into());
    }
    Ok(())
}

fn finite_result(value: FLOAT, name: &str) -> MathResult<FLOAT> {
    validate_scalar(value, name)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vec3(x: FLOAT, y: FLOAT, z: FLOAT) -> Map {
        [("x", x), ("y", y), ("z", z)]
            .into_iter()
            .map(|(key, value)| (key.into(), Dynamic::from(value)))
            .collect()
    }

    fn components(value: &Map) -> [FLOAT; 3] {
        ["x", "y", "z"].map(|key| value[key].as_float().unwrap())
    }

    #[test]
    fn vector_arithmetic_and_products() {
        let a = vec3(1.0, 2.0, 3.0);
        let b = vec3(4.0, 5.0, 6.0);
        assert_eq!(
            components(&ScriptMath::vec3(1.0, 2.0, 3.0).unwrap()),
            [1.0, 2.0, 3.0]
        );
        assert_eq!(
            components(&ScriptMath::add(a.clone(), b.clone()).unwrap()),
            [5.0, 7.0, 9.0]
        );
        assert_eq!(
            components(&ScriptMath::subtract(a.clone(), b.clone()).unwrap()),
            [-3.0, -3.0, -3.0]
        );
        assert_eq!(
            components(&ScriptMath::scale(a.clone(), 2.0).unwrap()),
            [2.0, 4.0, 6.0]
        );
        assert_eq!(ScriptMath::dot(a.clone(), b.clone()).unwrap(), 32.0);
        assert_eq!(
            components(&ScriptMath::cross(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0)).unwrap()),
            [0.0, 0.0, 1.0]
        );
        assert_eq!(ScriptMath::length(vec3(3.0, 4.0, 0.0)).unwrap(), 5.0);
        assert!((ScriptMath::distance(a, b).unwrap() - 3.0 * 3.0_f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn normalization_direction_and_lerp_cover_boundaries() {
        assert_eq!(
            components(&ScriptMath::normalize(vec3(0.0, 3.0, 4.0)).unwrap().unwrap()),
            [0.0, 0.6, 0.8]
        );
        assert!(
            ScriptMath::normalize(vec3(1e-13, 0.0, 0.0))
                .unwrap()
                .is_none()
        );
        assert_eq!(
            components(
                &ScriptMath::direction(vec3(1.0, 2.0, 3.0), vec3(1.0, 5.0, 7.0))
                    .unwrap()
                    .unwrap()
            ),
            [0.0, 0.6, 0.8]
        );
        assert!(
            ScriptMath::direction(vec3(1.0, 2.0, 3.0), vec3(1.0, 2.0, 3.0))
                .unwrap()
                .is_none()
        );
        let a = vec3(-2.0, 4.0, 7.0);
        let b = vec3(8.0, -6.0, 1.0);
        assert_eq!(
            components(&ScriptMath::lerp(a.clone(), b.clone(), 0.0).unwrap()),
            components(&a)
        );
        assert_eq!(
            components(&ScriptMath::lerp(a, b.clone(), 1.0).unwrap()),
            components(&b)
        );
        assert_eq!(
            components(
                &ScriptMath::lerp(vec3(0.0, 0.0, 0.0), vec3(10.0, 20.0, 30.0), 0.25).unwrap()
            ),
            [2.5, 5.0, 7.5]
        );
    }

    #[test]
    fn vector_results_are_fresh_maps() {
        let source = vec3(1.0, 2.0, 3.0);
        let mut result = ScriptMath::scale(source.clone(), 1.0).unwrap();
        result.insert("x".into(), Dynamic::from(99.0));
        assert_eq!(components(&source), [1.0, 2.0, 3.0]);
        assert_eq!(
            components(&ScriptMath::vec3(1.0, 2.0, 3.0).unwrap()),
            [1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn malformed_nonfinite_and_out_of_range_inputs_are_errors() {
        let mut missing = vec3(1.0, 2.0, 3.0);
        missing.remove("y");
        let mut extra = vec3(1.0, 2.0, 3.0);
        extra.insert("w".into(), Dynamic::from(4.0));
        let nonnumeric = [
            ("x".into(), Dynamic::from("no")),
            ("y".into(), Dynamic::from(0.0)),
            ("z".into(), Dynamic::from(0.0)),
        ]
        .into_iter()
        .collect();
        assert!(ScriptMath::length(missing).is_err());
        assert!(ScriptMath::length(extra).is_err());
        assert!(ScriptMath::length(nonnumeric).is_err());
        assert!(ScriptMath::length(vec3(FLOAT::INFINITY, 0.0, 0.0)).is_err());
        assert!(ScriptMath::length(vec3(FLOAT::NAN, 0.0, 0.0)).is_err());
        assert!(ScriptMath::length(vec3(100_001.0, 0.0, 0.0)).is_err());
        assert!(ScriptMath::vec3(0.0, 0.0, 100_001.0).is_err());
        assert!(ScriptMath::scale(vec3(1.0, 0.0, 0.0), FLOAT::NAN).is_err());
        assert!(ScriptMath::lerp(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0), -0.1).is_err());
        assert!(ScriptMath::lerp(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0), 1.1).is_err());
    }

    #[test]
    fn vector_operations_reject_overflowing_results() {
        let large = vec3(100_000.0, 100_000.0, 100_000.0);
        assert!(ScriptMath::add(large.clone(), large.clone()).is_err());
        assert!(ScriptMath::scale(vec3(100_000.0, 0.0, 0.0), 2.0).is_err());
        assert!(ScriptMath::cross(vec3(100_000.0, 0.0, 0.0), vec3(0.0, 100_000.0, 0.0)).is_err());
    }

    #[test]
    fn scalar_results_are_finite_without_spatial_component_bounds() {
        let large = vec3(100_000.0, 100_000.0, 100_000.0);
        let opposite = vec3(-100_000.0, -100_000.0, -100_000.0);
        assert_eq!(
            ScriptMath::dot(large.clone(), large.clone()).unwrap(),
            30_000_000_000.0
        );
        assert_eq!(
            ScriptMath::dot(large.clone(), opposite.clone()).unwrap(),
            -30_000_000_000.0
        );
        assert!(
            (ScriptMath::length(large.clone()).unwrap() - 100_000.0 * 3.0_f64.sqrt()).abs() < 1e-9
        );
        assert!(
            (ScriptMath::distance(large, opposite).unwrap() - 200_000.0 * 3.0_f64.sqrt()).abs()
                < 1e-9
        );
    }

    #[test]
    fn scale_factors_are_finite_without_spatial_component_bounds() {
        assert_eq!(
            components(&ScriptMath::scale(vec3(0.001, 0.0, 0.0), 1_000_000.0).unwrap()),
            [1000.0, 0.0, 0.0]
        );
        assert_eq!(
            components(&ScriptMath::scale(vec3(0.001, 0.0, 0.0), -1_000_000.0).unwrap()),
            [-1000.0, 0.0, 0.0]
        );
        assert_eq!(
            components(&ScriptMath::scale(vec3(0.0, 0.0, 0.0), FLOAT::MAX).unwrap()),
            [0.0; 3]
        );
        assert!(ScriptMath::scale(vec3(0.0, 0.0, 0.0), FLOAT::INFINITY).is_err());
        assert!(ScriptMath::scale(vec3(0.0, 0.0, 0.0), FLOAT::NAN).is_err());
    }

    #[test]
    fn normalize_and_direction_respect_exact_near_zero_boundary() {
        let above = FLOAT::from_bits(ZERO_EPSILON.to_bits() + 1);
        for magnitude in [0.0, 1e-13, ZERO_EPSILON] {
            assert!(
                ScriptMath::normalize(vec3(magnitude, 0.0, 0.0))
                    .unwrap()
                    .is_none()
            );
            assert!(
                ScriptMath::direction(vec3(0.0, 0.0, 0.0), vec3(magnitude, 0.0, 0.0))
                    .unwrap()
                    .is_none()
            );
        }
        assert_eq!(
            components(
                &ScriptMath::normalize(vec3(above, 0.0, 0.0))
                    .unwrap()
                    .unwrap()
            ),
            [1.0, 0.0, 0.0]
        );
        assert_eq!(
            components(
                &ScriptMath::direction(vec3(0.0, 0.0, 0.0), vec3(above, 0.0, 0.0))
                    .unwrap()
                    .unwrap()
            ),
            [1.0, 0.0, 0.0]
        );
    }

    #[test]
    fn malformed_maps_propagate_rhai_errors() {
        let mut engine = Engine::new();
        ScriptMath::register(&mut engine);
        let mut scope = rhai::Scope::new();
        scope.push("math", ScriptMath);
        scope.push("not_a_number", FLOAT::NAN);
        for (expression, expected) in [
            ("math.length(#{x: 1, z: 0})", "exactly numeric x, y, and z"),
            (
                "math.normalize(#{x: 1, y: 0, z: 0, w: 0})",
                "exactly numeric x, y, and z",
            ),
            (
                "math.direction(#{x: 0, y: 0, z: 0}, #{x: true, y: 0, z: 0})",
                "components must be numbers",
            ),
            (
                "math.length(#{x: not_a_number, y: 0, z: 0})",
                "components must be finite",
            ),
        ] {
            let error = engine
                .eval_with_scope::<Dynamic>(&mut scope, expression)
                .expect_err("invalid vectors should report an error through Rhai");
            assert!(
                error.to_string().contains(expected),
                "{expression}: {error}"
            );
        }
    }
    #[test]
    fn normalize_and_direction_return_maps_or_rhai_unit() {
        let mut engine = Engine::new();
        ScriptMath::register(&mut engine);
        let mut scope = rhai::Scope::new();
        scope.push("math", ScriptMath);

        let normalized = engine
            .eval_with_scope::<FLOAT>(&mut scope, "math.normalize(#{x: 0.0, y: 3.0, z: 4.0}).z")
            .expect("normalized value should be usable as a vector map");
        assert_eq!(normalized, 0.8);
        let direction = engine
            .eval_with_scope::<FLOAT>(
                &mut scope,
                "math.direction(#{x: 0, y: 2.0, z: 3}, #{x: 0.0, y: 5, z: 7.0}).y",
            )
            .expect("direction result should be usable as a vector map");
        assert_eq!(direction, 0.6);

        let normalized_zero = engine
            .eval_with_scope::<Dynamic>(&mut scope, "math.normalize(#{x: 0.0, y: 0.0, z: 0.0})")
            .expect("zero normalization should return unit");
        assert!(normalized_zero.is_unit());
        let coincident_direction = engine
            .eval_with_scope::<Dynamic>(
                &mut scope,
                "math.direction(#{x: 1, y: 2.0, z: 3}, #{x: 1.0, y: 2, z: 3.0})",
            )
            .expect("coincident points should return unit");
        assert!(coincident_direction.is_unit());
    }

    #[test]
    fn rhai_vector_maps_accept_integer_and_mixed_numeric_components() {
        let mut engine = Engine::new();
        ScriptMath::register(&mut engine);
        let mut scope = rhai::Scope::new();
        scope.push("math", ScriptMath);
        let length = engine
            .eval_with_scope::<FLOAT>(&mut scope, "math.length(#{x: 3, y: 4.0, z: 0})")
            .expect("integer and floating point components should be accepted");
        assert_eq!(length, 5.0);
    }

    #[test]
    fn methods_are_exposed_to_rhai() {
        let mut engine = Engine::new();
        ScriptMath::register(&mut engine);
        let mut scope = rhai::Scope::new();
        scope.push("math", ScriptMath);
        let value = engine
            .eval_with_scope::<FLOAT>(
                &mut scope,
                "math.dot(math.vec3(1.0, 2.0, 3.0), math.vec3(4.0, 5.0, 6.0))",
            )
            .unwrap();
        assert_eq!(value, 32.0);
    }
}
