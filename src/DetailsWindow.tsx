import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { DetailsCard } from "./components/DetailsCard";
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

  return (
    <main className="details-shell" style={{ padding: DETAILS_SHADOW_MARGIN }}>
      <DetailsCard cardRef={cardRef} details={details} />
    </main>
  );
}

export default DetailsWindow;
