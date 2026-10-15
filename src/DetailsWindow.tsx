import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import { listen } from "@tauri-apps/api/event";
import { DETAILS_CHANGED_EVENT, DETAILS_SHADOW_MARGIN, type ProviderDetails } from "./details";
import { pinWindowSize } from "./windowSize";
import "./App.css";

function DetailsWindow() {
  const [details, setDetails] = useState<ProviderDetails | null>(null);
  const cardRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const subscription = listen<ProviderDetails>(DETAILS_CHANGED_EVENT, ({ payload }) =>
      setDetails(payload),
    );
    return () => {
      subscription.then((unlisten) => unlisten()).catch(() => undefined);
    };
  }, []);

  useLayoutEffect(() => {
    const card = cardRef.current;
    if (!card) return;

    pinWindowSize(
      card.offsetWidth + DETAILS_SHADOW_MARGIN * 2,
      card.offsetHeight + DETAILS_SHADOW_MARGIN * 2,
    ).catch(() => undefined);
  }, [details]);

  if (!details) return null;

  const cardStyle = { "--provider-accent": details.accent } as CSSProperties;

  return (
    <main className="details-shell" style={{ padding: DETAILS_SHADOW_MARGIN }}>
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
        <span className="provider-orb-details-usage">
          <strong>{details.percentageLabel}</strong>
          <span>{details.detail}</span>
        </span>
        {details.weekly && (
          <span className="provider-orb-details-usage">
            <strong>{details.weekly.percentageLabel}</strong>
            <span>
              Weekly{details.weekly.resetLabel ? ` · ${details.weekly.resetLabel}` : ""}
            </span>
          </span>
        )}
        {(details.isMock || details.isStale) && (
          <span className="provider-orb-flags">
            {details.isMock && <span>mock</span>}
            {details.isStale && <span className="is-stale">cached</span>}
          </span>
        )}
      </div>
    </main>
  );
}

export default DetailsWindow;
