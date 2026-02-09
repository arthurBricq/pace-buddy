/**
 * Get the p value (fraction of MAS) based on race distance
 * @param distance_m Distance in meters
 * @returns p value (fraction of MAS)
 */
export function getPValue(distance_m: number): number {
  if (distance_m >= 1500 && distance_m <= 3000) {
    return 0.98; // 1500m-3k
  } else if (Math.abs(distance_m - 5000) < 100) {
    return 0.92; // 5k
  } else if (Math.abs(distance_m - 10000) < 100) {
    return 0.90; // 10k
  } else if (Math.abs(distance_m - 21100) < 500) {
    return 0.85; // Half-marathon (21.1k)
  } else if (Math.abs(distance_m - 42200) < 1000) {
    return 0.80; // Marathon (42.2k)
  } else {
    // Default: interpolate or use closest value
    if (distance_m < 5000) {
      return 0.95; // Between 3k and 5k
    } else if (distance_m < 10000) {
      return 0.91; // Between 5k and 10k
    } else if (distance_m < 21100) {
      return 0.875; // Between 10k and half-marathon
    } else if (distance_m < 42200) {
      return 0.825; // Between half-marathon and marathon
    } else {
      return 0.75; // Longer than marathon
    }
  }
}

/**
 * Calculate MAS (Maximum Aerobic Speed) in m/s from race distance and time
 * Formula: MAS (m/s) = (D_m / T_s) * (1 / p)
 * @param distance_m Distance in meters
 * @param time_s Time in seconds
 * @returns MAS in m/s
 */
export function calculateMAS(distance_m: number, time_s: number): number {
  if (time_s <= 0 || distance_m <= 0) {
    return 0;
  }
  const p = getPValue(distance_m);
  const averageSpeed = distance_m / time_s; // m/s
  return averageSpeed / p;
}

/**
 * Convert MAS from m/s to km/h
 * @param mas_ms MAS in m/s
 * @returns MAS in km/h
 */
export function masToKmh(mas_ms: number): number {
  return mas_ms * 3.6;
}

/**
 * Convert MAS from km/h to m/s
 * @param mas_kmh MAS in km/h
 * @returns MAS in m/s
 */
export function masToMps(mas_kmh: number): number {
  return mas_kmh / 3.6;
}
