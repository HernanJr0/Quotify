import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface RuntimeInfo {
  id: string;
  kind: "windows" | "linux" | "macos" | "wsl";
  name: string;
}

function App() {
  const [runtimes, setRuntimes] = useState<RuntimeInfo[]>([]);

  useEffect(() => {
    invoke<RuntimeInfo[]>("list_runtimes")
      .then(setRuntimes)
      .catch(() => setRuntimes([]));
  }, []);

  return (
    <main className="flex h-screen w-screen flex-col items-center justify-center gap-2 bg-neutral-950 text-neutral-100">
      <h1 className="text-lg font-semibold tracking-tight">Quotify</h1>
      <p className="text-sm text-neutral-400">Agent Usage HUD</p>

      <ul className="mt-4 flex flex-col items-center gap-1 text-xs text-neutral-300">
        {runtimes.map((rt) => (
          <li key={rt.id}>{rt.name}</li>
        ))}
      </ul>

      <p className="mt-6 text-xs text-neutral-600">Ctrl+Shift+U to show/hide</p>
    </main>
  );
}

export default App;
