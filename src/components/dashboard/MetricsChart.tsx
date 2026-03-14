import {
  ResponsiveContainer,
  LineChart,
  Line,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  CartesianGrid,
} from "recharts";

interface DataPoint {
  label: string;
  value: number;
  [key: string]: string | number;
}

interface MetricsChartProps {
  title: string;
  data: DataPoint[];
  dataKey: string;
  color: string;
  chartType?: "line" | "bar";
  height?: number;
  formatValue?: (value: number) => string;
}

export function MetricsChart({
  title,
  data,
  dataKey,
  color,
  chartType = "line",
  height = 200,
  formatValue,
}: MetricsChartProps) {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const tooltipFormatter = formatValue
    ? ((val: any) => [formatValue(Number(val ?? 0)), title]) as any
    : undefined;

  return (
    <div className="bg-gray-800/50 border border-gray-700 rounded-lg p-4">
      <h3 className="text-sm font-medium text-gray-300 mb-3">{title}</h3>
      <ResponsiveContainer width="100%" height={height}>
        {chartType === "bar" ? (
          <BarChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#2d2d3f" />
            <XAxis
              dataKey="label"
              tick={{ fontSize: 11, fill: "#6b7280" }}
              axisLine={{ stroke: "#374151" }}
            />
            <YAxis
              tick={{ fontSize: 11, fill: "#6b7280" }}
              axisLine={{ stroke: "#374151" }}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "#1f2937",
                border: "1px solid #374151",
                borderRadius: "8px",
                fontSize: "12px",
              }}
              formatter={tooltipFormatter}
            />
            <Bar dataKey={dataKey} fill={color} radius={[4, 4, 0, 0]} />
          </BarChart>
        ) : (
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#2d2d3f" />
            <XAxis
              dataKey="label"
              tick={{ fontSize: 11, fill: "#6b7280" }}
              axisLine={{ stroke: "#374151" }}
            />
            <YAxis
              tick={{ fontSize: 11, fill: "#6b7280" }}
              axisLine={{ stroke: "#374151" }}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "#1f2937",
                border: "1px solid #374151",
                borderRadius: "8px",
                fontSize: "12px",
              }}
              formatter={tooltipFormatter}
            />
            <Line
              type="monotone"
              dataKey={dataKey}
              stroke={color}
              strokeWidth={2}
              dot={{ fill: color, r: 3 }}
              activeDot={{ r: 5 }}
            />
          </LineChart>
        )}
      </ResponsiveContainer>
    </div>
  );
}
