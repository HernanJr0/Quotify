# Quotify

Overlay desktop leve para acompanhar, de relance, quanto ainda dá para usar de cada agente de IA: **Claude Code**, **Codex**, **Gemini** e **Grok**.

O Quotify fica sempre no topo, sem moldura, com uma bolinha por agente. Passe o mouse para ver o uso por janela de cota, a barra de preenchimento e o horário (e dia) do reset. Tudo roda local: sem conta, sem servidor, sem telemetria.

## Funcionalidades

- **Uso por janela:** cada agente mostra dois anéis concêntricos, semanal por fora e 5 horas por dentro. Provedores com uma janela só mostram um anel.
- **Card de detalhes no hover:** porcentagem, barra de preenchimento e reset de cada janela (só o horário se for hoje; dia da semana e data nos demais casos).
- **Windows e WSL2:** detecta o agente em cada ambiente e junta as instalações que usam a mesma conta num único módulo.
- **Atalho global** `Ctrl+Shift+U` para mostrar/esconder, ícone na bandeja e botões flutuantes (atualizar, fixar, aparência).
- **Local-first:** o último uso válido fica num SQLite local e é exibido, marcado como em cache, quando um provedor falha.

## Provedores

| Provedor    | Fonte do dado                                                                                | Estado                          |
| ----------- | -------------------------------------------------------------------------------------------- | ------------------------------- |
| Claude Code | `get_usage` do próprio CLI (sessão de 5h e semanal), sem chamar modelo e sem ler credenciais | real                            |
| Codex       | app-server oficial do CLI                                                                    | real                            |
| Gemini      | —                                                                                            | mock, aparece rotulado como tal |
| Grok        | —                                                                                            | mock, aparece rotulado como tal |

O Quotify nunca inventa cotas: quando não há fonte confiável, o provedor aparece como mock ou indisponível.

## Instalação

O Quotify é distribuído como instalador Windows (`.msi`/`.exe`), gerado pelo GitHub Actions a cada push na `main` e sem assinatura de código. Baixe o artefato `quotify-windows-latest` da última execução em [Actions](https://github.com/HernanJr0/Quotify/actions).

Os agentes que você quer acompanhar precisam estar instalados e autenticados (`claude`, `codex`).

## Desenvolvimento

Requisitos: Node 22, Rust estável e, no Windows, o Visual Studio Build Tools com o workload "Desktop development with C++".

```bash
npm install
npm run tauri dev
```

### Scripts

- `npm run dev`: servidor Vite (só o frontend)
- `npm run tauri dev`: app Tauri completo em desenvolvimento
- `npm run build`: build de produção do frontend
- `npm run tauri build`: build completo do app desktop
- `npm run lint`: ESLint
- `npm run format` / `npm run format:check`: Prettier

Testes do backend:

```bash
cd src-tauri && cargo test
```

## Stack

Tauri 2 (Rust), React 19, TypeScript, Vite e Tailwind CSS 4. O banco local é SQLite via `rusqlite`.

## Estrutura

- `src/`: interface (overlay, janela de detalhes e de aparência)
- `src-tauri/src/runtime/`: execução de comandos por ambiente (Windows, WSL, Linux, macOS)
- `src-tauri/src/providers/`: detecção e adapters de cada provedor
- `src-tauri/src/storage.rs`: persistência local

## Plataformas

O alvo é o Windows (incluindo WSL2) e o CI builda só para ele. A camada de runtime tem código para Linux e macOS, mas esses sistemas não são testados nem distribuídos.
