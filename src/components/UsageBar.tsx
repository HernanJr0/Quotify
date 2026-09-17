// Thresholds match docs/PLAN.md section 33.
function statusColor(percentage: number): string {
  if (percentage >= 95) return "bg-red-500";
  if (percentage >= 85) return "bg-orange-500";
  if (percentage >= 70) return "bg-yellow-500";
  return "bg-emerald-500";
}

interface UsageBarProps {
  percentage: number;
}

export function UsageBar({ percentage }: UsageBarProps) {
  const clamped = Math.min(100, Math.max(0, percentage));

  return (
    <div className="h-1.5 w-full overflow-hidden rounded-full bg-neutral-800">
      <div
        className={`h-full rounded-full ${statusColor(clamped)}`}
        style={{ width: `${clamped}%` }}
      />
    </div>
  );
}
