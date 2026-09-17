import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ProviderCard } from "./components/ProviderCard";
import type { ProviderInstallation } from "./types/provider";
import type { RuntimeInfo } from "./types/runtime";
import "./App.css";

function App() {
  const [runtimes, setRuntimes] = useState<RuntimeInfo[]>([]);
  const [installations, setInstallations] = useState<ProviderInstallation[]>([]);

  useEffect(() => {
    invoke<RuntimeInfo[]>("list_runtimes")
      .then(setRuntimes)
      .catch(() => setRuntimes([]));
    invoke<ProviderInstallation[]>("list_providers")
      .then(setInstallations)
      .catch(() => setInstallations([]));
  }, []);

  return (
    <main className="flex h-screen w-screen flex-col gap-3 bg-neutral-950 p-4 text-neutral-100">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">Quotify</h1>
          <p className="text-xs text-neutral-500">Agent Usage HUD</p>
        </div>
        <span className="text-[11px] text-neutral-600">Ctrl+Shift+U</span>
      </header>

      {installations.length > 0 ? (
        <div className="grid grid-cols-2 gap-2 overflow-y-auto">
          {installations.map((installation) => (
            <ProviderCard key={installation.id} installation={installation} />
          ))}
        </div>
      ) : (
        <p className="text-xs text-neutral-500">
          No agents detected in: {runtimes.map((rt) => rt.name).join(", ") || "no environments"}
        </p>
      )}
    </main>
  );
}

export default App;
