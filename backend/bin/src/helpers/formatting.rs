pub fn format_duration_hms(total_seconds: i32) -> String {
    let seconds = total_seconds.max(0);
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{h}h{m:02}m{s:02}s")
    } else {
        format!("{m}m{s:02}s")
    }
}

pub fn format_pace_from_seconds(seconds_per_km: f64) -> String {
    if !seconds_per_km.is_finite() || seconds_per_km <= 0.0 {
        return "N/A".to_string();
    }
    let minutes = (seconds_per_km / 60.0).floor() as i64;
    let seconds = (seconds_per_km.round() as i64) % 60;
    format!("{minutes}:{seconds:02}/km")
}

pub fn format_pace_from_activity(distance_m: f64, moving_time_s: i32) -> String {
    if distance_m <= 0.0 || moving_time_s <= 0 {
        return "N/A".to_string();
    }
    let seconds_per_km = moving_time_s as f64 / (distance_m / 1000.0);
    format_pace_from_seconds(seconds_per_km)
}
