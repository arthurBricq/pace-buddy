export interface User {
  id: string;
  username: string;
  display_name: string;
  created_at: string;
}

export type ActivityTag = 'normal' | 'intervals' | 'race';

export interface Activity {
  id: string;
  user_id: string;
  strava_id: number;
  name: string;
  sport_type: string;
  start_date: string;
  elapsed_time: number;
  moving_time: number;
  distance: number;
  total_elevation_gain: number;
  average_speed: number;
  max_speed: number;
  average_heartrate: number | null;
  max_heartrate: number | null;
  average_cadence: number | null;
  average_watts: number | null;
  calories: number | null;
  tag: ActivityTag;
  summary_polyline: string | null;
  workout_type: number | null;
  streams_loaded: boolean;
  created_at: string;
}

export type StreamType =
  | 'time'
  | 'distance'
  | 'latlng'
  | 'altitude'
  | 'heartrate'
  | 'cadence'
  | 'watts'
  | 'velocity_smooth'
  | 'moving';

export interface ActivityStream {
  activity_id: string;
  stream_type: StreamType;
  data_json: string;
}

export interface ActivityDetail {
  activity: Activity;
  streams: ActivityStream[];
}

export interface StravaStatus {
  linked: boolean;
  athlete_id?: number;
}

// Interval parsing types

export type SegmentKind = 'Warmup' | 'Work' | 'Recovery' | 'Cooldown' | 'Pause' | 'Steady' | 'Unknown';
export type RecoveryStyle = 'Jog' | 'Walk' | 'Stop' | 'Unknown';

export interface Segment {
  kind: SegmentKind;
  start_t: number;
  end_t: number;
  duration_s: number;
  distance_m: number;
  avg_speed_mps: number;
  speed_std_mps: number;
  max_speed_mps: number;
  avg_hr: number | null;
  avg_cadence: number | null;
}

export interface Rep {
  work: Segment;
  recovery: Segment | null;
  rep_index: number;
  set_index: number | null;
  distance_m: number;
  duration_s: number;
  avg_pace_s_per_km: number;
  avg_speed_mps: number;
  pace_std: number;
  pct_mas: number | null;
  steadiness: number;
  fade: number;
  recovery_style: RecoveryStyle | null;
}

export interface IntervalResult {
  segments: Segment[];
  reps: Rep[];
  is_interval_workout: boolean;
  interval_score: number;
  threshold_speed_mps: number;
  cluster_low_mps: number;
  cluster_high_mps: number;
}

export interface Training {
  id: string;
  user_id: string;
  name: string;
  description: string | null;
  created_at: string;
}
