export type RuntimeKind = "windows" | "linux" | "macos" | "wsl";

export interface RuntimeInfo {
  id: string;
  kind: RuntimeKind;
  name: string;
}
