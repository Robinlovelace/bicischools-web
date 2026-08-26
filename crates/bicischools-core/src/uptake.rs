/// Propensity to Cycle Tool (PCT) Go Dutch scenario for travel to school.
/// Based on Goodman et al. (2019) and the PCT package implementation.

/// Default parameters for the Go Dutch school uptake model (uptake_pct_godutch_school2)
pub const DEFAULT_ALPHA: f64 = -7.178 + 3.574; // -3.604
pub const DEFAULT_D1: f64 = -1.87 + 0.3438;    // -1.5262
pub const DEFAULT_D2: f64 = 5.961;
pub const DEFAULT_H1: f64 = -0.529;
pub const DEFAULT_H2: f64 = -0.63;

/// Calculates the probability of cycling for a school trip given distance (in meters) and gradient (percentage or ratio).
///
/// # Arguments
/// * `distance_m` - Route distance in meters
/// * `gradient_pct` - Average gradient in percent (e.g., 2.5 for 2.5% slope). If gradient < 0.1, it is treated as a ratio and converted to %.
pub fn calculate_go_dutch_school(distance_m: f64, gradient_pct: f64) -> f64 {
    let mut dist_km = distance_m / 1000.0;
    if dist_km > 30.0 {
        dist_km = 30.0;
    }
    if dist_km <= 0.0 {
        return 0.0;
    }

    let mut grad = gradient_pct;
    if grad > 0.0 && grad < 0.1 {
        grad *= 100.0;
    }

    let logit = DEFAULT_ALPHA
        + (DEFAULT_D1 * dist_km)
        + (DEFAULT_D2 * dist_km.sqrt())
        + (DEFAULT_H1 * (grad + DEFAULT_H2));

    inv_logit(logit)
}

/// Inverse logit (sigmoid) function: 1 / (1 + exp(-x))
#[inline]
pub fn inv_logit(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_dutch_values() {
        // Distance 2km (2000m), gradient 1%
        let p = calculate_go_dutch_school(2000.0, 1.0);
        // Expect around 0.8289 matching R pct::uptake_pct_godutch_school2(2, 1)
        assert!((p - 0.8289).abs() < 0.01, "Got {p}");
    }
}
