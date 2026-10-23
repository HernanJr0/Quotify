interface UsageRingProps {
  className?: string;
  // `null` draws only the dashed track, for a window the provider isn't reporting.
  percentage: number | null;
  radius?: number;
  status: string;
}

const DEFAULT_RING_RADIUS = 46;

export function UsageRing({
  className = "",
  percentage,
  radius = DEFAULT_RING_RADIUS,
  status,
}: UsageRingProps) {
  const circumference = 2 * Math.PI * radius;
  const clamped = Math.min(100, Math.max(0, percentage ?? 0));
  const offset = circumference * (1 - clamped / 100);

  return (
    <svg
      className={`usage-ring ${status} ${percentage == null ? "is-idle" : ""} ${className}`}
      viewBox="0 0 100 100"
      aria-hidden="true"
    >
      <circle className="usage-ring-track" cx="50" cy="50" r={radius} />
      {percentage != null && (
        <circle
          className="usage-ring-value"
          cx="50"
          cy="50"
          r={radius}
          strokeDasharray={circumference}
          strokeDashoffset={offset}
        />
      )}
    </svg>
  );
}
