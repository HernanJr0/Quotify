# Quotify

Agent Usage HUD — overlay desktop para acompanhar consumo e limites de agentes de IA (Claude Code, Codex, Gemini, Grok).

Ver [docs/PLAN.md](docs/PLAN.md) para visão geral, arquitetura e fases de desenvolvimento.

## Stack

- Tauri 2 (Rust)
- React + TypeScript + Vite
- Tailwind CSS

## Desenvolvimento

```bash
npm install
npm run tauri dev
```

## Scripts

- `npm run dev` — Vite dev server (frontend isolado)
- `npm run tauri dev` — app Tauri completo em modo desenvolvimento
- `npm run build` — build de produção do frontend
- `npm run tauri build` — build completo do app desktop
- `npm run lint` — ESLint
- `npm run format` — Prettier (write)
- `npm run format:check` — Prettier (check)

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
