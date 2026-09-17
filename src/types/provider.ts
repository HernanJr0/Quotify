export type ProviderKind = "claude" | "codex" | "gemini" | "grok";

export interface ProviderInstallation {
  id: string;
  provider: ProviderKind;
  providerName: string;
  runtimeId: string;
  runtimeName: string;
  executable: string;
}
