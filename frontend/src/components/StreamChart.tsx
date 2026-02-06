import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import type { ActivityStream } from '../types';

const STREAM_COLORS: Record<string, string> = {
  heartrate: '#ef4444',
  altitude: '#22c55e',
  velocity_smooth: '#3b82f6',
  cadence: '#f59e0b',
  watts: '#8b5cf6',
};

const STREAM_LABELS: Record<string, string> = {
  heartrate: 'Heart Rate (bpm)',
  altitude: 'Elevation (m)',
  velocity_smooth: 'Speed (m/s)',
  cadence: 'Cadence (spm)',
  watts: 'Power (W)',
};

interface Props {
  streams: ActivityStream[];
  timeStream?: ActivityStream;
  distanceStream?: ActivityStream;
}

export default function StreamChart({ streams, distanceStream }: Props) {
  const chartableStreams = streams.filter(
    (s) => !['time', 'distance', 'latlng'].includes(s.stream_type),
  );

  if (chartableStreams.length === 0) return null;

  const distanceData: number[] = distanceStream
    ? JSON.parse(distanceStream.data_json)
    : [];

  return (
    <div className="space-y-6">
      {chartableStreams.map((stream) => {
        const data: number[] = JSON.parse(stream.data_json);
        const chartData = data.map((value, i) => ({
          distance: distanceData[i] ? (distanceData[i] / 1000).toFixed(2) : i,
          value,
        }));

        const color = STREAM_COLORS[stream.stream_type] || '#6b7280';
        const label = STREAM_LABELS[stream.stream_type] || stream.stream_type;

        return (
          <div key={stream.stream_type} className="bg-white rounded-lg shadow p-4">
            <h3 className="text-sm font-medium text-gray-700 mb-2">{label}</h3>
            <ResponsiveContainer width="100%" height={200}>
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis
                  dataKey="distance"
                  tick={{ fontSize: 10 }}
                  label={{ value: 'km', position: 'insideBottomRight', offset: -5, fontSize: 10 }}
                />
                <YAxis tick={{ fontSize: 10 }} />
                <Tooltip />
                <Line
                  type="monotone"
                  dataKey="value"
                  stroke={color}
                  dot={false}
                  strokeWidth={1.5}
                />
              </LineChart>
            </ResponsiveContainer>
          </div>
        );
      })}
    </div>
  );
}
