interface UsageRingProps {
  className?: string;
  percentage: number;
  status: string;
}

const RING_RADIUS = 43;
const RING_CIRCUMFERENCE = 2 * Math.PI * RING_RADIUS;

export function UsageRing({ className = "", percentage, status }: UsageRingProps) {
  const clamped = Math.min(100, Math.max(0, percentage));
  const offset = RING_CIRCUMFERENCE * (1 - clamped / 100);

  return (
    <svg className={`usage-ring ${status} ${className}`} viewBox="0 0 100 100" aria-hidden="true">
      <circle className="usage-ring-track" cx="50" cy="50" r={RING_RADIUS} />
      <circle
        className="usage-ring-value"
        cx="50"
        cy="50"
        r={RING_RADIUS}
        strokeDasharray={RING_CIRCUMFERENCE}
        strokeDashoffset={offset}
      />
    </svg>
  );
}
