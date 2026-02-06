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
