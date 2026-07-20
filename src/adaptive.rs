/**
 * BlissMixer: Use Bliss analysis results to create music mixes
 *
 * Copyright (c) 2022-2026 Craig Drummond <craig.p.drummond@gmail.com>
 * GPLv3 license.
 *
 **/
use crate::tree;
use bliss_mixer_core::scoring::{mean_feature_vector, select_adaptive_matrix, AdaptiveAlgorithm};
use ndarray::Array2;

pub struct MatrixSelection {
    pub matrix: Option<Array2<f32>>,
    pub algorithm_name: String,
}

pub fn select_matrix(
    seed_raw_metrics: &[[f32; tree::DIMENSIONS]],
    learned_matrix: Option<&Array2<f32>>,
    learnedblend: u16,
) -> MatrixSelection {
    let selection = select_adaptive_matrix(seed_raw_metrics, learned_matrix, learnedblend)
        .expect("learned blend is validated by the mixer CLI");

    match &selection.algorithm {
        AdaptiveAlgorithm::Blended { learned_percent } => log::debug!(
            "Blending learned (alpha={:.2}) and variance matrices from {} seeds",
            *learned_percent as f32 / 100.0,
            seed_raw_metrics.len()
        ),
        AdaptiveAlgorithm::VarianceBased => log::debug!(
            "Using variance-based adaptive weight matrix from {} seeds",
            seed_raw_metrics.len()
        ),
        AdaptiveAlgorithm::LearnedMatrix if seed_raw_metrics.len() == 1 => {
            log::debug!("Using learned Mahalanobis matrix for single seed")
        }
        _ => {}
    }

    if let Some(error) = &selection.variance_failure {
        if selection.matrix.is_some() {
            log::warn!(
                "Failed to build variance-based matrix: {}. Falling back to learned matrix.",
                error
            );
        } else {
            log::warn!(
                "Failed to build variance-based matrix: {}. Falling back to standard algorithm.",
                error
            );
        }
    }

    MatrixSelection {
        matrix: selection.matrix,
        algorithm_name: selection.algorithm.to_string(),
    }
}

pub fn mean_metrics(
    seed_raw_metrics: &[[f32; tree::DIMENSIONS]],
) -> Option<[f32; tree::DIMENSIONS]> {
    mean_feature_vector(seed_raw_metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bliss_audio::playlist::variance_based_weight_matrix;
    use ndarray::Array1;

    fn identity() -> Array2<f32> {
        Array2::eye(tree::DIMENSIONS)
    }

    fn feature(value: f32) -> [f32; tree::DIMENSIONS] {
        std::array::from_fn(|index| value + index as f32 / 10.0)
    }

    #[test]
    fn empty_and_single_seed_rules_are_stable() {
        let learned = identity();
        let empty = select_matrix(&[], Some(&learned), 20);
        assert_eq!(empty.algorithm_name, "none");
        assert!(empty.matrix.is_none());

        let one = select_matrix(&[feature(1.0)], Some(&learned), 20);
        assert_eq!(one.algorithm_name, "learned-matrix");
        assert_eq!(one.matrix.unwrap(), learned);

        let no_learned = select_matrix(&[feature(1.0)], None, 20);
        assert_eq!(no_learned.algorithm_name, "none");
        assert!(no_learned.matrix.is_none());
    }

    #[test]
    fn variance_and_blend_endpoints_are_stable() {
        let seeds = [feature(0.0), feature(2.0)];
        let seed_arrays: Vec<_> = seeds
            .iter()
            .map(|seed| Array1::from_vec(seed.to_vec()))
            .collect();
        let variance = variance_based_weight_matrix(&seed_arrays).unwrap();
        let learned = identity() * 3.0;

        let variance_only = select_matrix(&seeds, None, 20);
        assert_eq!(variance_only.algorithm_name, "variance-based");
        assert_eq!(variance_only.matrix.unwrap(), variance);

        let zero = select_matrix(&seeds, Some(&learned), 0);
        assert_eq!(zero.algorithm_name, "blended(learned=0%)");
        assert_eq!(zero.matrix.unwrap(), variance);

        let hundred = select_matrix(&seeds, Some(&learned), 100);
        assert_eq!(hundred.algorithm_name, "blended(learned=100%)");
        assert_eq!(hundred.matrix.unwrap(), learned);
    }

    #[test]
    fn mean_is_the_arithmetic_seed_mean() {
        let mean = mean_metrics(&[feature(0.0), feature(2.0)]).unwrap();
        for (actual, expected) in mean.iter().zip(feature(1.0)) {
            assert!((actual - expected).abs() < 1e-6);
        }
        assert!(mean_metrics(&[]).is_none());
    }
}
