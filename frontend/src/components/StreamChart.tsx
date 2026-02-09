import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ReferenceArea,
} from 'recharts';
import type { ActivityStream, Segment, SegmentKind } from '../types';

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

const SEGMENT_COLORS: Record<SegmentKind, string> = {
  Work: '#3b82f6',
  Recovery: '#9ca3af',
  Warmup: '#22c55e',
  Cooldown: '#22c55e',
  Pause: '#f97316',
  Steady: '#6b7280',
  Unknown: '#6b7280',
};

interface SegmentArea {
  x1: number;
  x2: number;
  fill: string;
}

function buildSegmentAreas(
  segments: Segment[],
  timeData: number[],
  distanceData: number[],
): SegmentArea[] {
  if (timeData.length === 0 || distanceData.length === 0) return [];

  return segments.map((seg) => {
    // Find the distance values corresponding to segment time boundaries
    let x1Km = 0;
    let x2Km = 0;
    for (let i = 0; i < timeData.length; i++) {
      if (timeData[i] <= seg.start_t) x1Km = distanceData[i] / 1000;
      if (timeData[i] <= seg.end_t) x2Km = distanceData[i] / 1000;
    }
    return {
      x1: x1Km,
      x2: x2Km,
      fill: SEGMENT_COLORS[seg.kind] || '#6b7280',
    };
  });
}

interface Props {
  streams: ActivityStream[];
  timeStream?: ActivityStream;
  distanceStream?: ActivityStream;
  segments?: Segment[];
}

export default function StreamChart({ streams, distanceStream, timeStream, segments }: Props) {
  const chartableStreams = streams.filter(
    (s) => !['time', 'distance', 'latlng'].includes(s.stream_type),
  );

  if (chartableStreams.length === 0) return null;

  const distanceData: number[] = distanceStream
    ? JSON.parse(distanceStream.data_json)
    : [];

  const timeData: number[] = timeStream
    ? JSON.parse(timeStream.data_json)
    : [];

  const segmentAreas = segments && segments.length > 0
    ? buildSegmentAreas(segments, timeData, distanceData)
    : [];

  return (
    <div className="space-y-6">
      {chartableStreams.map((stream) => {
        const data: number[] = JSON.parse(stream.data_json);
        const chartData = data.map((value, i) => ({
          distance: distanceData[i] != null ? distanceData[i] / 1000 : i,
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
                  type="number"
                  domain={['dataMin', 'dataMax']}
                  tickFormatter={(v: number) => v.toFixed(1)}
                  tick={{ fontSize: 10 }}
                  label={{ value: 'km', position: 'insideBottomRight', offset: -5, fontSize: 10 }}
                />
                <YAxis tick={{ fontSize: 10 }} />
                <Tooltip />
                {segmentAreas.map((area, i) => (
                  <ReferenceArea
                    key={i}
                    x1={area.x1}
                    x2={area.x2}
                    fill={area.fill}
                    fillOpacity={0.12}
                    strokeOpacity={0}
                  />
                ))}
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
