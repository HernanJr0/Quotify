// Thresholds match docs/PLAN.md section 33.
function statusColor(percentage: number): string {
  if (percentage >= 95) return "is-critical";
  if (percentage >= 85) return "is-high";
  if (percentage >= 70) return "is-warning";
  return "is-ok";
}

interface UsageBarProps {
  percentage: number;
}

export function UsageBar({ percentage }: UsageBarProps) {
  const clamped = Math.min(100, Math.max(0, percentage));

  return (
    <div className="usage-track">
      <div className={"usage-fill " + statusColor(clamped)} style={{ width: clamped + "%" }} />
    </div>
  );
}
