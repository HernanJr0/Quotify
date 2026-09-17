import type { ProviderInstallation } from "../types/provider";
import { UsageBar } from "./UsageBar";

// Fase 5 uses mocked usage — real numbers arrive with the adapters built in
// Fase 8-9 (docs/PLAN.md). Values mirror the illustrative overlay in
// section 2 so the mock feels representative rather than arbitrary.
const MOCK_USAGE: Record<string, { percentage: number; period: string; reset: string }> = {
  claude: { percentage: 71, period: "5h rolling", reset: "reset 14:20" },
  codex: { percentage: 43, period: "weekly", reset: "reset Monday" },
  gemini: { percentage: 52, period: "daily", reset: "reset 00:00" },
  grok: { percentage: 82, period: "daily", reset: "reset Monday" },
};

interface ProviderCardProps {
  installation: ProviderInstallation;
}

export function ProviderCard({ installation }: ProviderCardProps) {
  const mock = MOCK_USAGE[installation.provider] ?? {
    percentage: 0,
    period: "unknown",
    reset: "",
  };

  return (
    <div className="flex flex-col gap-1.5 rounded-lg border border-neutral-800 bg-neutral-900 p-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium text-neutral-100">{installation.providerName}</span>
        <span className="text-xs text-neutral-500">
          {mock.percentage}% <span className="text-neutral-700">(mock)</span>
        </span>
      </div>

      <UsageBar percentage={mock.percentage} />

      <span className="text-[11px] text-neutral-500">
        {mock.period} · {mock.reset}
      </span>
      <span className="text-[11px] text-neutral-600">{installation.runtimeName}</span>
    </div>
  );
}
