// Utilities for tests and examples.

use std::borrow::Borrow;

use bevy_math::Vec3;
use rand::{prelude::Distribution, Rng};

// Returns a Vec3 with each element sampled from the given distribution.
pub fn random_vec3<R: Rng + ?Sized, T: Borrow<f32>, D: Distribution<T>>(
    rng: &mut R,
    dist: D,
) -> Vec3 {
    Vec3::new(
        *rng.sample(&dist).borrow(),
        *rng.sample(&dist).borrow(),
        *rng.sample(&dist).borrow(),
    )
}
