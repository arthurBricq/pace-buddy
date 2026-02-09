/// Compute the arithmetic mean of a slice. Returns 0.0 for empty input.
pub fn mean(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    data.iter().sum::<f64>() / data.len() as f64
}

/// Compute the population standard deviation.
pub fn std_dev(data: &[f64]) -> f64 {
    if data.len() < 2 {
        return 0.0;
    }
    let m = mean(data);
    let variance = data.iter().map(|x| (x - m).powi(2)).sum::<f64>() / data.len() as f64;
    variance.sqrt()
}

/// Compute the coefficient of variation (std_dev / mean). Returns 0 if mean is ~0.
pub fn cv(data: &[f64]) -> f64 {
    let m = mean(data);
    if m.abs() < 1e-12 {
        return 0.0;
    }
    std_dev(data) / m
}

/// Compute the median of a slice. Returns 0.0 for empty input.
/// Does not mutate the input; allocates a sorted copy.
pub fn median(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut sorted: Vec<f64> = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    }
}

/// Compute the p-th percentile (0..100) using linear interpolation.
pub fn percentile(data: &[f64], p: f64) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut sorted: Vec<f64> = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    let rank = (p / 100.0) * (n - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    let frac = rank - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

/// Compute the interquartile range (Q3 - Q1).
pub fn iqr(data: &[f64]) -> f64 {
    percentile(data, 75.0) - percentile(data, 25.0)
}

/// Apply a rolling median filter of given window size (must be odd, forced odd if even).
/// Returns a vector of the same length, with edges handled by shrinking the window.
pub fn rolling_median(data: &[f64], window: usize) -> Vec<f64> {
    if data.is_empty() {
        return vec![];
    }
    let w = if window % 2 == 0 { window + 1 } else { window };
    let half = w / 2;
    let n = data.len();
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let lo = if i >= half { i - half } else { 0 };
        let hi = if i + half < n { i + half + 1 } else { n };
        result.push(median(&data[lo..hi]));
    }
    result
}

/// Apply a rolling mean filter of given window size.
/// Returns a vector of the same length, with edges handled by shrinking the window.
pub fn rolling_mean(data: &[f64], window: usize) -> Vec<f64> {
    if data.is_empty() || window <= 1 {
        return data.to_vec();
    }
    let half = window / 2;
    let n = data.len();
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let lo = if i >= half { i - half } else { 0 };
        let hi = if i + half < n { i + half + 1 } else { n };
        let sum: f64 = data[lo..hi].iter().sum();
        result.push(sum / (hi - lo) as f64);
    }
    result
}

/// K-means clustering with k=2 on 1D data.
/// Returns (centroid_low, centroid_high, boundary) where boundary is the midpoint.
/// Runs at most `max_iter` iterations.
pub fn kmeans_k2(data: &[f64], max_iter: usize) -> (f64, f64, f64) {
    if data.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    if data.len() == 1 {
        return (data[0], data[0], data[0]);
    }

    // Initialize centroids as min and max
    let mut sorted: Vec<f64> = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut c_lo = sorted[sorted.len() / 4]; // 25th percentile
    let mut c_hi = sorted[3 * sorted.len() / 4]; // 75th percentile

    for _ in 0..max_iter {
        let boundary = (c_lo + c_hi) / 2.0;

        let mut sum_lo = 0.0;
        let mut count_lo = 0usize;
        let mut sum_hi = 0.0;
        let mut count_hi = 0usize;

        for &x in data {
            if x <= boundary {
                sum_lo += x;
                count_lo += 1;
            } else {
                sum_hi += x;
                count_hi += 1;
            }
        }

        let new_lo = if count_lo > 0 {
            sum_lo / count_lo as f64
        } else {
            c_lo
        };
        let new_hi = if count_hi > 0 {
            sum_hi / count_hi as f64
        } else {
            c_hi
        };

        if (new_lo - c_lo).abs() < 1e-9 && (new_hi - c_hi).abs() < 1e-9 {
            break;
        }
        c_lo = new_lo;
        c_hi = new_hi;
    }

    let boundary = (c_lo + c_hi) / 2.0;
    (c_lo, c_hi, boundary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean() {
        assert_eq!(mean(&[]), 0.0);
        assert!((mean(&[1.0, 2.0, 3.0]) - 2.0).abs() < 1e-10);
        assert!((mean(&[10.0]) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_std_dev() {
        assert_eq!(std_dev(&[]), 0.0);
        assert_eq!(std_dev(&[5.0]), 0.0);
        // std_dev of [1,2,3,4,5] = sqrt(2.0) ≈ 1.4142
        let sd = std_dev(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!((sd - (2.0_f64).sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_median() {
        assert_eq!(median(&[]), 0.0);
        assert!((median(&[3.0, 1.0, 2.0]) - 2.0).abs() < 1e-10);
        assert!((median(&[4.0, 1.0, 3.0, 2.0]) - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_percentile() {
        assert_eq!(percentile(&[], 50.0), 0.0);
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile(&data, 0.0) - 1.0).abs() < 1e-10);
        assert!((percentile(&data, 100.0) - 5.0).abs() < 1e-10);
        assert!((percentile(&data, 50.0) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_iqr() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let q1 = percentile(&data, 25.0);
        let q3 = percentile(&data, 75.0);
        assert!((iqr(&data) - (q3 - q1)).abs() < 1e-10);
    }

    #[test]
    fn test_rolling_median() {
        let data = [1.0, 10.0, 2.0, 10.0, 3.0];
        let smoothed = rolling_median(&data, 3);
        assert_eq!(smoothed.len(), 5);
        // middle elements: median of [1,10,2]=2, [10,2,10]=10, [2,10,3]=3
        assert!((smoothed[1] - 2.0).abs() < 1e-10);
        assert!((smoothed[2] - 10.0).abs() < 1e-10);
        assert!((smoothed[3] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_kmeans_k2_bimodal() {
        // Two clear clusters around 3 and 8
        let data: Vec<f64> = vec![
            2.8, 3.0, 3.2, 2.9, 3.1, // low cluster ~3
            7.8, 8.0, 8.2, 7.9, 8.1, // high cluster ~8
        ];
        let (c_lo, c_hi, boundary) = kmeans_k2(&data, 50);
        assert!((c_lo - 3.0).abs() < 0.3);
        assert!((c_hi - 8.0).abs() < 0.3);
        assert!(boundary > 4.0 && boundary < 7.0);
    }

    #[test]
    fn test_cv() {
        assert_eq!(cv(&[]), 0.0);
        let data = [10.0, 10.0, 10.0];
        assert!((cv(&data)).abs() < 1e-10);
    }
}
