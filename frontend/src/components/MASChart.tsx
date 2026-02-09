import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Dot,
} from 'recharts';
import type { MASEstimate } from '../types';

interface MASChartProps {
  estimates: MASEstimate[];
}

function formatDate(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

function CustomDot(props: any) {
  const { cx, cy } = props;
  return (
    <Dot
      cx={cx}
      cy={cy}
      r={6}
      fill="#3b82f6"
      stroke="#fff"
      strokeWidth={2}
    />
  );
}

export default function MASChart({ estimates }: MASChartProps) {
  if (estimates.length === 0) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <p className="text-gray-500 text-center">No race data available</p>
      </div>
    );
  }

  // Sort by date
  const sortedEstimates = [...estimates].sort(
    (a, b) => new Date(a.date).getTime() - new Date(b.date).getTime(),
  );

  // Convert dates to timestamps for linear x-axis
  const chartData = sortedEstimates.map((est) => {
    const date = new Date(est.date);
    return {
      date: est.date,
      timestamp: date.getTime(), // Use timestamp for linear scale
      dateLabel: formatDate(est.date),
      mas: est.mas_kmh,
      activityName: est.activity_name,
      distance: (est.distance_m / 1000).toFixed(2),
    };
  });

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h3 className="text-lg font-semibold mb-4">MAS Estimate Over Time</h3>
      <ResponsiveContainer width="100%" height={400}>
        <LineChart data={chartData} margin={{ top: 5, right: 20, bottom: 5, left: 0 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
          <XAxis
            dataKey="timestamp"
            type="number"
            domain={['dataMin', 'dataMax']}
            stroke="#6b7280"
            style={{ fontSize: '12px' }}
            angle={-45}
            textAnchor="end"
            height={80}
            tickFormatter={(value) => {
              const date = new Date(value);
              return formatDate(date.toISOString());
            }}
          />
          <YAxis
            label={{ value: 'MAS (km/h)', angle: -90, position: 'insideLeft' }}
            stroke="#6b7280"
            style={{ fontSize: '12px' }}
          />
          <Tooltip
            content={({ active, payload }) => {
              if (active && payload && payload.length > 0) {
                const data = payload[0].payload;
                return (
                  <div className="bg-white border border-gray-200 rounded-md shadow-lg p-3">
                    <p className="font-medium text-sm">{data.activityName}</p>
                    <p className="text-xs text-gray-600">
                      {new Date(data.date).toLocaleDateString()}
                    </p>
                    <p className="text-sm font-semibold text-blue-600">
                      MAS: {data.mas.toFixed(2)} km/h
                    </p>
                    <p className="text-xs text-gray-500">
                      Distance: {data.distance} km
                    </p>
                  </div>
                );
              }
              return null;
            }}
          />
          <Line
            type="monotone"
            dataKey="mas"
            stroke="#3b82f6"
            strokeWidth={2}
            dot={<CustomDot />}
            activeDot={{ r: 8 }}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
