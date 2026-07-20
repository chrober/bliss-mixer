/**
 * BlissMixer: Use Bliss analysis results to create music mixes
 *
 * Copyright (c) 2022-2026 Craig Drummond <craig.p.drummond@gmail.com>
 * GPLv3 license.
 *
 **/
use crate::tree;
use bliss_audio::playlist::variance_based_weight_matrix;
use ndarray::{Array1, Array2};

pub struct MatrixSelection {
    pub matrix: Option<Array2<f32>>,
    pub algorithm_name: String,
}

pub fn select_matrix(
    seed_raw_metrics: &[[f32; tree::DIMENSIONS]],
    learned_matrix: Option<&Array2<f32>>,
    learnedblend: u16,
) -> MatrixSelection {
    if seed_raw_metrics.len() >= 2 {
        let seed_arrays: Vec<Array1<f32>> = seed_raw_metrics
            .iter()
            .map(|metrics| Array1::from_vec(metrics.to_vec()))
            .collect();
        return match (variance_based_weight_matrix(&seed_arrays), learned_matrix) {
            (Ok(variance), Some(learned)) => {
                // Preserve exact endpoints to avoid IEEE 754 inf*0.0=NaN if
                // a matrix implementation yields non-finite variance entries.
                let matrix = if learnedblend == 100 {
                    learned.clone()
                } else if learnedblend == 0 {
                    variance
                } else {
                    let alpha = learnedblend as f32 / 100.0;
                    learned * alpha + &variance * (1.0 - alpha)
                };
                log::debug!(
                    "Blending learned (alpha={:.2}) and variance matrices from {} seeds",
                    learnedblend as f32 / 100.0,
                    seed_raw_metrics.len()
                );
                MatrixSelection {
                    matrix: Some(matrix),
                    algorithm_name: format!("blended(learned={}%)", learnedblend),
                }
            }
            (Ok(variance), None) => {
                log::debug!(
                    "Using variance-based adaptive weight matrix from {} seeds",
                    seed_raw_metrics.len()
                );
                MatrixSelection {
                    matrix: Some(variance),
                    algorithm_name: "variance-based".to_string(),
                }
            }
            (Err(error), Some(learned)) => {
                log::warn!(
                    "Failed to build variance-based matrix: {}. Falling back to learned matrix.",
                    error
                );
                MatrixSelection {
                    matrix: Some(learned.clone()),
                    algorithm_name: "learned-matrix".to_string(),
                }
            }
            (Err(error), None) => {
                log::warn!(
                    "Failed to build variance-based matrix: {}. Falling back to standard algorithm.",
                    error
                );
                MatrixSelection {
                    matrix: None,
                    algorithm_name: "none".to_string(),
                }
            }
        };
    }

    if !seed_raw_metrics.is_empty() {
        if let Some(learned) = learned_matrix {
            log::debug!("Using learned Mahalanobis matrix for single seed");
            return MatrixSelection {
                matrix: Some(learned.clone()),
                algorithm_name: "learned-matrix".to_string(),
            };
        }
    }

    MatrixSelection {
        matrix: None,
        algorithm_name: "none".to_string(),
    }
}

pub fn mean_metrics(
    seed_raw_metrics: &[[f32; tree::DIMENSIONS]],
) -> Option<[f32; tree::DIMENSIONS]> {
    if seed_raw_metrics.is_empty() {
        return None;
    }
    let mut mean = [0.0; tree::DIMENSIONS];
    for raw in seed_raw_metrics {
        for (mean_value, raw_value) in mean.iter_mut().zip(raw) {
            *mean_value += raw_value;
        }
    }
    for value in &mut mean {
        *value /= seed_raw_metrics.len() as f32;
    }
    Some(mean)
}

#[cfg(test)]
mod tests {
    use super::*;

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
