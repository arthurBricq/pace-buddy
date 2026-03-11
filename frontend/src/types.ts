export interface User {
  id: string;
  username: string;
  display_name: string;
  email?: string | null;
  created_at: string;
  mas_current: number | null; // Current MAS estimate in m/s
  quota_balance_usd: number;
}

export interface QuotaRequestRecord {
  id: string;
  user_id: string;
  status: 'pending' | 'approved' | 'rejected';
  requested_at: string;
  resolved_at: string | null;
  granted_amount_usd: number | null;
}

export interface QuotaStatus {
  balance_usd: number;
  has_pending_request: boolean;
  requests: QuotaRequestRecord[];
}

export type ActivityTag = 'normal' | 'intervals' | 'race' | 'long_run';

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
  streams_fetched_at: string | null;
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

export interface ActivityLap {
  activity_id: string;
  lap_index: number;
  name: string;
  start_date: string;
  elapsed_time: number;
  moving_time: number;
  distance: number;
  average_speed: number;
  max_speed: number;
  total_elevation_gain: number;
  average_heartrate: number | null;
  max_heartrate: number | null;
}

export interface ActivityDetail {
  activity: Activity;
  streams: ActivityStream[];
  laps: ActivityLap[];
}

export interface StravaStatus {
  linked: boolean;
  athlete_id?: number;
}

export interface ExpensiveRequest {
  id: string;
  type: 'insight' | 'chat';
  title: string;
  model: string | null;
  cost: number;
  created_at: string;
  training_id?: string | null;
}

export interface AiCostSummary {
  total_cost: number;
  expensive_requests: ExpensiveRequest[];
}

// Interval parsing types

export type IntervalAlgorithm = 'speed_based' | 'manual_laps';

export type SegmentKind = 'Warmup' | 'Work' | 'Recovery' | 'Cooldown' | 'Pause' | 'Steady' | 'Unknown';
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
  recovery_duration_s: number | null;
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

export interface IntervalResponse extends IntervalResult {
  algorithm: IntervalAlgorithm;
}

export interface Training {
  id: string;
  user_id: string;
  name: string;
  description: string | null;
  start_date: string | null;
  end_date: string | null;
  race_distance: string | null;
  race_objectif: string | null;
  created_at: string;
}

export interface RunningStats {
  total_distance_m: number;
  total_time_s: number;
  total_elevation_m: number;
  avg_speed_mps: number | null;
  activity_count: number;
  interval_count: number | null;
}

export interface ProfileResponse {
  user: User;
  stats: {
    ytd: RunningStats;
    last_year: RunningStats;
    all_time: RunningStats;
  };
}

export interface TrainingInsightResponse {
  id: string;
  display_label: string;
  full_prompt: string;
  response: string;
}

export interface TrainingInsightRecord {
  id: string;
  training_id: string;
  prompt_type: string;
  display_label: string;
  full_prompt: string;
  response: string;
  model?: string | null;
  cost?: number | null;
  created_at: string;
}

export interface ModelInfo {
  id: string;
  name: string;
  description?: string | null;
  pricing?: {
    prompt: string;
    completion: string;
  } | null;
  context_length?: number | null;
}

export type ModelCostCategory = 'economical' | 'standard' | 'expensive';

export interface ModelCostTier {
  model_id: string;
  model_name: string;
  category: ModelCostCategory;
  computed_at: string;
}

export interface AiChat {
  id: string;
  user_id: string;
  training_id: string | null;
  source_insight_id: string | null;
  source_insight_cost: number;
  title: string;
  model: string;
  conversation_length?: number | null;
  created_at: string;
  updated_at: string;
}

export interface AiChatMessage {
  id: string;
  chat_id: string;
  role: string;
  content: string;
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  cost: number;
  context_label: string | null;
  created_at: string;
}

export interface ChatResponse {
  chat: AiChat;
  messages: AiChatMessage[];
  total_cost: number;
  total_tokens: number;
}

export interface ChatListItem {
  id: string;
  title: string;
  model: string;
  training_id: string | null;
  message_count: number;
  total_cost: number;
  created_at: string;
  updated_at: string;
}

export interface MASEstimate {
  date: string;
  mas_ms: number;
  mas_kmh: number;
  activity_id: string;
  activity_name: string;
  distance_m: number;
  time_s: number;
}
