import type { CSSProperties, Ref } from "react";
import type { ProviderDetails } from "../details";

interface DetailsCardProps {
  cardRef?: Ref<HTMLDivElement>;
  details: ProviderDetails;
}

export function DetailsCard({ cardRef, details }: DetailsCardProps) {
  const cardStyle = { "--provider-accent": details.accent } as CSSProperties;

  return (
    <div
      className={"provider-details-card " + details.status}
      ref={cardRef}
      role="tooltip"
      style={cardStyle}
    >
      <span className="provider-orb-details-heading">
        <strong>{details.providerName}</strong>
        <span>{details.runtimeLabel}</span>
      </span>
      {details.windows.length === 0 && (
        <span className="provider-orb-details-usage">
          <strong>{details.percentageLabel}</strong>
          <span>{details.detail}</span>
        </span>
      )}
      {details.windows.map((window) => (
        <span className={"provider-orb-details-window " + window.status} key={window.label}>
          <span className="provider-orb-details-usage">
            <strong>{window.percentageLabel}</strong>
            <span>
              {window.label} ·{" "}
              {window.percentage == null ? "No active window" : (window.resetLabel ?? "—")}
            </span>
          </span>
          <span className="provider-orb-details-bar">
            <span style={{ width: `${Math.min(100, Math.max(0, window.percentage ?? 0))}%` }} />
          </span>
        </span>
      ))}
      {(details.isMock || details.isStale) && (
        <span className="provider-orb-flags">
          {details.isMock && <span>mock</span>}
          {details.isStale && <span className="is-stale">cached</span>}
        </span>
      )}
    </div>
  );
}
