# Quotify

## 1. Visão geral

**Quotify** é uma aplicação desktop leve que funciona como um overlay/HUD para desenvolvedores acompanharem rapidamente o consumo e os limites de ferramentas de agentes de IA.

Providers iniciais:

* Claude Code
* OpenAI Codex
* Gemini
* Grok

O Quotify deve permitir visualizar rapidamente:

* uso atual;
* porcentagem consumida, quando disponível;
* limite restante;
* período da cota;
* horário ou data de reset;
* origem da informação;
* ambiente onde a ferramenta foi detectada;
* estado da integração;
* última atualização.

O projeto deve funcionar bem nos principais ambientes usados por desenvolvedores:

* Windows;
* Linux;
* Windows + WSL2.

O sistema deve ser local-first, leve e independente de infraestrutura externa.

---

# 2. Objetivo principal

O Quotify deve responder em poucos segundos:

> Quanto ainda posso usar de cada agente agora?

A experiência principal é um pequeno overlay acionado através de atalho global.

Exemplo:

```text
┌────────────────────────────────────────────┐
│ Quotify                         ↻ 09:51    │
├────────────────────┬───────────────────────┤
│ Claude             │ Codex                 │
│ ███████░░░ 71%     │ ████░░░░░░ 43%       │
│ reset 14:00        │ weekly                │
│ WSL · Ubuntu       │ Windows               │
├────────────────────┼───────────────────────┤
│ Gemini             │ Grok                  │
│ █████░░░░░ 52%     │ ████████░░ 82%       │
│ daily              │ reset Monday          │
│ Windows            │ Linux                 │
└────────────────────┴───────────────────────┘
```

---

# 3. Escopo do MVP

O MVP deve:

1. iniciar como aplicação desktop;
2. permanecer na bandeja do sistema;
3. abrir através de hotkey global;
4. possuir overlay compacto;
5. detectar ambientes disponíveis;
6. detectar agentes instalados;
7. mostrar um card por provider;
8. permitir múltiplos runtimes;
9. coletar uso através de adapters;
10. armazenar snapshots localmente;
11. atualizar automaticamente;
12. continuar funcionando mesmo quando um provider falhar.

Não implementar inicialmente:

* sincronização em nuvem;
* contas do Quotify;
* backend remoto;
* dashboard web;
* colaboração entre usuários;
* gerenciamento de prompts;
* execução de agentes;
* roteamento automático de tarefas;
* suporte SSH;
* suporte Docker;
* sistema público de plugins;
* scraping de dashboards como primeira estratégia.

---

# 4. Sistemas suportados

## Windows

Suporte principal.

Distribuição esperada:

```text
Quotify.exe
```

ou instalador:

```text
.msi
```

---

## Linux

Suporte nativo.

Ambiente gráfico suportado no MVP: X11 (e XWayland). Wayland nativo tem suporte limitado a always-on-top e hotkeys globais no Tauri 2; fica registrado como limitação conhecida, sem bloquear o MVP.

Formatos possíveis:

```text
.AppImage
.deb
.rpm
```

O formato inicial pode ser AppImage + deb.

---

## macOS

Suporte principal, incluído desde o início do MVP — RuntimeManager e detecção de macOS entram junto com Windows/Linux/WSL a partir da Fase 2/3 (ver seção 47).

Formato de distribuição:

```text
.dmg
```

Sem assinatura de código nem notarização no MVP (ver seção 59). O usuário libera o app manualmente no Gatekeeper na primeira execução.

---

## Windows + WSL2

O Quotify roda como aplicação Windows.

Ele deve conseguir descobrir e consultar ferramentas instaladas dentro das distribuições WSL.

Exemplo:

```text
Windows

Quotify.exe
    │
    ├── Codex Windows
    ├── Gemini Windows
    │
    └── WSL
        ├── Ubuntu
        │   ├── Claude Code
        │   └── Codex
        │
        └── Debian
            └── Gemini
```

Não executar uma segunda instância do Quotify dentro do WSL.

---

# 5. Stack

## Desktop

Tauri 2

Motivos:

* baixo consumo;
* suporte multiplataforma;
* integração com tray;
* hotkeys globais;
* always-on-top;
* execução de processos;
* integração nativa;
* frontend separado da lógica de sistema.

---

## Frontend

* React
* TypeScript
* Vite

---

## Estilização

* Tailwind CSS

---

## Backend desktop

Rust através do Tauri.

Responsabilidades:

* processos;
* filesystem;
* runtimes;
* detecção de CLI;
* WSL;
* SQLite;
* segurança;
* collectors;
* adapters.

---

## Persistência

SQLite.

---

# 6. Princípio arquitetural

O Quotify não deve assumir que:

```text
provider = programa instalado no host
```

Um provider pode existir em diferentes runtimes.

Exemplo:

```text
Codex
├── Windows
└── WSL Ubuntu
```

Por isso devemos separar:

```text
Provider
Runtime
Installation
Account
Usage
```

---

# 7. Modelo conceitual

```text
Provider
    │
    │ possui
    ▼
Provider Installation
    │
    │ roda dentro de
    ▼
Runtime
```

Exemplo:

```text
Provider
OpenAI Codex

Installation #1
Windows

Installation #2
WSL Ubuntu
```

Ambas podem inclusive utilizar a mesma conta.

---

# 8. Arquitetura geral

```text
                       ┌─────────────────────┐
                       │      Quotify UI     │
                       │   React + Tauri     │
                       └──────────┬──────────┘
                                  │
                                  ▼
                          Usage Aggregator
                                  │
                           Provider Registry
                                  │
               ┌──────────────────┼──────────────────┐
               ▼                  ▼                  ▼
         ClaudeAdapter       CodexAdapter      GeminiAdapter
               │                  │                  │
               └──────────────────┼──────────────────┘
                                  │
                                  ▼
                           Runtime Manager
                                  │
                   ┌──────────────┼──────────────┐
                   ▼              ▼              ▼
            WindowsRuntime   LinuxRuntime    WslRuntime
                                                │
                                                ▼
                                             wsl.exe
                                  ┌─────────────┼────────────┐
                                  ▼             ▼            ▼
                               Ubuntu         Debian        Arch
```

---

# 9. Runtime Manager

Criar uma camada responsável por descobrir e representar ambientes de execução.

Interface conceitual:

```ts
interface Runtime {
  id: string;

  type: "windows" | "linux" | "wsl";

  name: string;

  execute(command: CommandRequest): Promise<CommandResult>;

  which(binary: string): Promise<string | null>;

  exists(path: string): Promise<boolean>;

  readFile(path: string): Promise<string>;

  getHomeDirectory(): Promise<string>;
}
```

A implementação real será feita em Rust.

---

# 10. Tipos de Runtime

## WindowsRuntime

Representa o Windows host.

Responsável por:

* executar `.exe`;
* resolver PATH;
* procurar executáveis;
* acessar arquivos Windows.

---

## LinuxRuntime

Representa Linux nativo.

Responsável por:

* executar binários;
* resolver PATH;
* acessar `$HOME`;
* manipular filesystem Linux.

---

## MacRuntime

Representa macOS nativo.

Responsável por:

* executar binários;
* resolver PATH, incluindo diretórios comuns de instalação como Homebrew (`/opt/homebrew/bin`, `/usr/local/bin`);
* acessar `$HOME`;
* manipular filesystem macOS.

---

## WslRuntime

Representa uma distro específica.

Exemplo:

```text
WSL Ubuntu-24.04
```

O runtime deve executar comandos através do Windows:

```bash
wsl.exe -d Ubuntu-24.04 -- <command>
```

Cada distribuição é um runtime diferente.

Exemplo:

```text
wsl:Ubuntu
wsl:Debian
wsl:Arch
```

---

# 11. Descoberta de WSL

No Windows:

```bash
wsl.exe --list --quiet
```

Resultado possível:

```text
Ubuntu
Debian
Arch
```

O Quotify deve transformar isso em:

```text
RuntimeManager

Windows

WSL Ubuntu
WSL Debian
WSL Arch
```

O sistema deve ignorar distribuições indisponíveis ou quebradas sem falhar.

---

# 12. Command Runner

Providers nunca devem executar diretamente comandos do SO.

Errado:

```rust
Command::new("claude")
```

Correto:

```text
runtime.execute(...)
```

Interface conceitual:

```ts
interface CommandRequest {
  executable: string;

  args: string[];

  timeout?: number;

  env?: Record<string, string>;
}
```

Resultado:

```ts
interface CommandResult {
  exitCode: number;

  stdout: string;

  stderr: string;

  durationMs: number;
}
```

---

# 13. Shell independence

Não assumir:

```text
bash
zsh
fish
powershell
cmd
```

Sempre que possível executar diretamente:

```text
executable
+
args
```

Exemplo:

```text
claude
["status"]
```

E não:

```text
bash -c "claude status"
```

Isso reduz:

* problemas de quoting;
* escaping;
* incompatibilidade;
* risco de command injection.

---

# 14. Descoberta de executáveis

Nunca assumir caminhos fixos.

Linux:

```text
/usr/bin
/usr/local/bin
```

Windows:

```text
Program Files
AppData
```

não devem ser considerados fonte única.

Um desenvolvedor pode usar:

* npm;
* pnpm;
* bun;
* brew;
* apt;
* nvm;
* mise;
* asdf;
* curl installers;
* instalações manuais.

Usar:

```text
runtime.which("claude")
runtime.which("codex")
runtime.which("gemini")
```

No Windows utilizar resolução equivalente do PATH.

---

# 15. Provider Installation

Modelo:

```ts
interface ProviderInstallation {
  id: string;

  provider: ProviderId;

  runtimeId: string;

  executable: string;

  detectedAt: string;

  version?: string;
}
```

Exemplo:

```text
Claude Code

Runtime:
WSL Ubuntu

Executable:
/home/user/.local/bin/claude

Version:
2.x
```

---

# 16. Provider Registry

Responsável por descobrir providers disponíveis.

Fluxo:

```text
RuntimeManager
      │
      ▼
list runtimes
      │
      ▼
ProviderRegistry
      │
      ├── procura claude
      ├── procura codex
      ├── procura gemini
      └── procura grok
```

Resultado:

```text
Windows
├── Codex
└── Gemini

WSL Ubuntu
├── Claude
└── Codex

WSL Debian
└── nenhum
```

---

# 17. Adapters

Cada provider possui adapter próprio.

Interface:

```ts
interface UsageProviderAdapter {
  provider: ProviderId;

  detect(runtime: Runtime): Promise<ProviderInstallation | null>;

  fetchUsage(
    installation: ProviderInstallation
  ): Promise<ProviderUsage>;

  diagnostics(
    installation: ProviderInstallation
  ): Promise<ProviderDiagnostics>;
}
```

Importante:

O adapter conhece:

```text
Claude
```

mas não deve precisar conhecer detalhes de:

```text
Windows
Linux
WSL
```

Esses detalhes pertencem ao Runtime.

---

# 18. Providers iniciais

Estrutura:

```text
providers/

  claude/
    adapter

  codex/
    adapter

  gemini/
    adapter

  grok/
    adapter
```

Cada um completamente independente.

Falha em:

```text
Gemini
```

não pode afetar:

```text
Claude
Codex
Grok
```

---

# 19. Modelo de Usage

```ts
type ProviderId =
  | "claude"
  | "codex"
  | "gemini"
  | "grok";

type UsageStatus =
  | "ok"
  | "warning"
  | "critical"
  | "unknown"
  | "unavailable"
  | "error";

type UsageUnit =
  | "percentage"
  | "tokens"
  | "requests"
  | "credits"
  | "messages"
  | "unknown";

type UsagePeriod =
  | "hourly"
  | "rolling"
  | "daily"
  | "weekly"
  | "monthly"
  | "unknown";

interface ProviderUsage {
  provider: ProviderId;

  installationId: string;

  runtimeId: string;

  status: UsageStatus;

  percentage?: number;

  used?: number;

  remaining?: number;

  limit?: number;

  unit?: UsageUnit;

  period?: UsagePeriod;

  periodDescription?: string;

  resetAt?: string;

  updatedAt: string;

  source: UsageSource;

  error?: string;
}
```

---

# 20. Não normalizar demais

Providers podem retornar métricas diferentes.

Exemplo:

```text
Claude
71%

Codex
52% weekly

Gemini
800 requests remaining

Grok
23 credits remaining
```

O Quotify não deve inventar:

```text
percentage
```

quando essa informação não puder ser calculada corretamente.

---

# 21. Account

É importante separar:

```text
installation
```

de:

```text
account
```

Porque isto pode acontecer:

```text
Codex

Windows
└── usuário A

WSL Ubuntu
└── usuário A
```

As duas instalações podem usar a mesma quota.

No MVP, correlação automática de contas não é obrigatória.

Estrutura deve apenas permitir evolução futura:

```ts
interface ProviderAccount {
  id: string;

  provider: ProviderId;

  displayName?: string;

  externalAccountId?: string;
}
```

---

# 22. Estratégia de coleta de usage

Usar esta ordem de preferência:

## 1. API oficial

Melhor opção.

Buscar:

* quota;
* usage;
* credits;
* remaining;
* reset;
* rate limit.

---

## 2. CLI oficial

Procurar comandos como:

```text
status
usage
quota
limits
account
```

---

## 3. Estado local

Investigar:

```text
~/.claude
~/.codex
~/.gemini
```

e equivalentes.

No Windows:

```text
%APPDATA%
%LOCALAPPDATA%
%USERPROFILE%
```

---

## 4. Endpoint utilizado pela própria ferramenta

Pode ser considerado se:

* for estável;
* for seguro;
* usar autenticação local;
* não exigir captura insegura de credenciais.

---

## 5. Scraping

Último recurso.

Nunca acoplar scraping diretamente ao frontend.

---

# 23. WSL e arquivos

Evitar depender diretamente de:

```text
\\wsl$\Ubuntu\...
```

Sempre que possível consultar arquivos através do próprio runtime:

```text
WslRuntime.readFile(...)
```

Isso preserva:

* permissões;
* HOME;
* links simbólicos;
* comportamento Linux;
* compatibilidade.

---

# 24. Spike obrigatório de providers

Antes de implementar coleta real, criar:

```text
docs/provider-research.md
```

Formato:

```text
# Claude Code

CLI encontrada:
sim/não

Ambientes testados:
Windows
Linux
WSL

Método de autenticação:
...

Método de usage:
...

Reset disponível:
...

Método escolhido:
...

Fallback:
...

Confiabilidade:
A/B/C/D
```

Repetir para:

```text
Claude
Codex
Gemini
Grok
```

---

# 25. Classificação das integrações

```text
A
API oficial ou mecanismo documentado

B
CLI/local state confiável

C
Integração parcial

D
Sem método confiável
```

Não inventar dados para providers classificados como D.

---

# 26. Usage Aggregator

Serviço central:

```ts
class UsageAggregator {
  refreshAll(): Promise<ProviderUsage[]>;

  refreshProvider(provider: ProviderId): Promise<ProviderUsage[]>;

  getCurrent(): ProviderUsage[];

  getHistory(provider: ProviderId): Promise<UsageSnapshot[]>;
}
```

Responsabilidades:

* chamar adapters;
* gerenciar runtimes;
* normalizar resultados;
* cache;
* timeout;
* persistência;
* erros;
* evitar refresh duplicado.

No MVP, `refreshAll()` executa os providers/runtimes de forma sequencial (não paralela), priorizando simplicidade e previsibilidade. Paralelismo com limite de concorrência é uma otimização futura, não um requisito do MVP.

---

# 27. Refresh

Default:

```text
5 minutos
```

Configuração futura:

```text
1 min
5 min
10 min
15 min
30 min
```

Também:

```text
Refresh All
```

e refresh individual.

---

# 28. Timeout

Cada coleta deve possuir timeout.

Default:

```text
10 segundos
```

Em caso de timeout:

```text
status: error
```

Mas manter último dado válido disponível.

Exemplo:

```text
Claude

71%

Last successful update:
09:45

Current status:
refresh timeout
```

---

# 29. SQLite

Tabelas principais:

## runtimes

```text
id
type
name
metadata
detected_at
last_seen
```

---

## provider_installations

```text
id
provider
runtime_id
executable
version
detected_at
last_seen
```

---

## provider_usage_snapshots

```text
id
provider
installation_id
runtime_id
timestamp
percentage
used
remaining
limit
unit
period
reset_at
status
source
```

---

## provider_state

```text
provider
installation_id
last_success
last_error
last_refresh
```

---

# 30. Segurança

Nunca persistir credenciais em plaintext.

Não incluir:

```text
tokens
cookies
API keys
session tokens
```

em:

```text
logs
SQLite
frontend
erros
diagnostics
```

Se futuramente for necessário armazenar segredo:

usar mecanismo seguro do SO.

Windows:

```text
Credential Manager
```

Linux:

```text
Secret Service / keyring
```

---

# 31. Interface principal

Overlay aproximadamente:

```text
400–500px
```

Características:

* dark;
* frameless;
* always-on-top;
* compacto;
* sem taskbar;
* leve transparência;
* animações discretas.

---

# 32. Provider Card

Mostrar:

```text
Provider
Usage
Progress
Period
Reset
Runtime
Last update
```

Exemplo:

```text
Claude

███████░░░ 71%

5h rolling
reset 14:20

WSL · Ubuntu

updated 1m ago
```

---

# 33. Status visual

Quando existir percentage:

```text
0–69
normal

70–84
warning

85–94
high

95–100
critical
```

Não aplicar classificação baseada em porcentagem quando ela não existir.

Quando não houver `percentage` (ex: créditos, requests, tokens — ver seção 20), exibir o valor bruto em um badge neutro, sem cores de warning/critical. Não inventar limiares arbitrários para métricas não percentuais no MVP.

---

# 34. Múltiplas instalações

Se provider estiver em múltiplos ambientes:

```text
Codex
```

ao clicar:

```text
Codex

Windows
✓ detected

WSL Ubuntu
✓ detected
```

Uma instalação pode ser marcada como:

```text
Preferred
```

---

# 35. Runtime preferido

Configuração por provider:

```text
Claude

○ Windows
○ WSL Ubuntu
○ WSL Debian
```

No MVP não existe resolução automática real entre múltiplas instalações: quando um provider tiver mais de uma instalação detectada, o usuário deve escolher manualmente qual runtime usar. "Automatic" só é implícito quando existe exatamente uma instalação (não há ambiguidade a resolver).

Lógica de priorização automática entre instalações concorrentes (ex: nativo vs WSL) fica para depois do MVP.

---

# 36. Diagnostics

Tela:

```text
Claude Code

Provider:
claude

Runtime:
WSL Ubuntu

Executable:
/home/user/.local/bin/claude

Version:
...

Adapter:
ClaudeAdapter

Usage source:
CLI

Last latency:
342ms

Last success:
09:48

Last error:
none
```

Nunca mostrar credenciais.

---

# 37. Tela de ambientes

Adicionar:

```text
Environments
```

Exemplo:

```text
Windows
✓ available

  Codex
  Gemini

WSL Ubuntu
✓ available

  Claude
  Codex

WSL Debian
✓ available

  No agents detected
```

Essa tela será especialmente útil para debug.

---

# 38. Tray

Menu:

```text
Open Quotify

Refresh All

Environments

Settings

Quit
```

Double click:

```text
Open Quotify
```

---

# 39. Hotkey

Default:

```text
Ctrl + Shift + U
```

Ação:

```text
show/hide
```

Configurável posteriormente.

---

# 40. Startup

Configuração:

```text
Launch Quotify on startup
```

Default:

```text
false
```

---

# 41. Settings

## General

```text
Launch on startup

Refresh interval

Global shortcut

Close on blur
```

---

## Providers

```text
Claude
enabled

Preferred runtime:
Automatic

Codex
enabled

Preferred runtime:
Windows
```

---

## Environments

```text
Windows
enabled

WSL Ubuntu
enabled

WSL Debian
disabled
```

---

# 42. Override manual

Permitir instalação customizada:

```text
Add provider installation
```

Exemplo:

```text
Provider:
Claude

Runtime:
WSL Ubuntu

Executable:
/home/user/bin/claude
```

Isso cobre instalações fora do PATH.

---

# 43. Histórico

Snapshots devem existir desde o MVP.

Visualização pode entrar depois.

Futuramente:

```text
24h
7d
30d
```

---

# 44. Most Available

Feature futura:

```text
Most available
```

Baseada somente em quota disponível.

Exemplo:

```text
Claude   29% available
Codex    57%
Gemini   82%
Grok     18%

Most available:
Gemini
```

Não significa melhor agente.

---

# 45. Notificações

Fase posterior.

Exemplo:

```text
Notify at:

80%
90%
95%
```

Cada threshold só deve disparar uma vez por janela.

---

# 46. Organização sugerida

```text
quotify/

  src/

    components/
      ProviderCard.tsx
      ProviderDetails.tsx
      UsageBar.tsx
      RuntimeBadge.tsx
      StatusBadge.tsx

    features/
      overlay/
      providers/
      environments/
      diagnostics/
      settings/
      history/

    services/
      usage.ts
      runtimes.ts
      providers.ts

    stores/
      usageStore.ts
      runtimeStore.ts
      settingsStore.ts

    types/
      usage.ts
      provider.ts
      runtime.ts

    App.tsx


  src-tauri/

    src/

      runtime/
        mod.rs
        manager.rs
        command.rs

        windows.rs
        linux.rs
        wsl.rs

      providers/
        mod.rs
        registry.rs

        claude/
          mod.rs

        codex/
          mod.rs

        gemini/
          mod.rs

        grok/
          mod.rs

      services/
        usage.rs
        database.rs
        diagnostics.rs

      models/
        runtime.rs
        provider.rs
        usage.rs

      commands/
        runtime_commands.rs
        provider_commands.rs
        usage_commands.rs

      main.rs


  docs/
    architecture.md
    provider-research.md
    runtime-design.md

  tests/
    fixtures/

  README.md
```

---

# 47. Fases de desenvolvimento

## Fase 0 — Bootstrap

Criar:

* Tauri 2;
* React;
* TypeScript;
* Vite;
* Tailwind;
* lint;
* formatter.

Validar build:

```text
Windows
Linux
macOS
```

---

# Fase 1 — Shell desktop

Implementar:

* janela frameless;
* always-on-top;
* tray;
* hotkey;
* show/hide;
* close on blur.

Usar dados fake.

---

# Fase 2 — Runtime layer

Antes dos providers reais:

implementar:

```text
Runtime
RuntimeManager
CommandRunner
```

Criar:

```text
WindowsRuntime
LinuxRuntime
MacRuntime
WslRuntime
```

Ordem de implementação: como o desenvolvimento inicial ocorre em Windows + WSL2, `WindowsRuntime` e `WslRuntime` são implementados e validados nativamente primeiro nesta fase. `LinuxRuntime` e `MacRuntime` entram com a mesma interface e cobertura de teste unitário, mas sua validação end-to-end (execução real de comandos) fica pendente até haver acesso a uma máquina Linux e macOS reais — isso não bloqueia a Fase 2, mas é um risco de validação registrado (ver seção 59).

---

# Fase 3 — Runtime discovery

Windows:

detectar:

```text
Windows
WSL distros
```

Linux:

detectar:

```text
Native Linux
```

macOS:

detectar:

```text
Native macOS
```

Mostrar ambientes na UI.

---

# Fase 4 — Provider Registry

Detectar:

```text
claude
codex
gemini
grok
```

em cada runtime.

Ainda sem consultar usage.

Resultado:

```text
Windows
Codex ✓

WSL Ubuntu
Claude ✓
Codex ✓
```

---

# Fase 5 — UI definitiva do overlay

Cards devem usar dados mockados, mas instalações reais.

Mostrar:

```text
Provider
Runtime
Usage mock
```

---

# Fase 6 — Provider Research

Criar:

```text
docs/provider-research.md
```

Investigar cada provider em:

```text
Windows
Linux
WSL
```

quando aplicável.

---

# Fase 7 — Adapter framework

Criar contrato:

```text
UsageProviderAdapter
```

Criar primeiro:

```text
MockAdapter
```

Validar arquitetura.

---

# Fase 8 — Primeiro provider real

Escolher provider mais confiável após research.

Implementar completo:

```text
detect
fetch usage
normalize
timeout
errors
diagnostics
```

---

# Fase 9 — Segundo provider

Adicionar próximo provider confiável.

Meta inicial do MVP:

```text
2 providers reais
```

---

# Fase 10 — SQLite

Criar:

```text
runtime storage
installations
snapshots
state
```

Usar migrations versionadas simples desde o início (ex: arquivos SQL numerados aplicados na inicialização), mesmo no MVP — evita reescrever schema manualmente conforme o modelo evolui.

---

# Fase 11 — Settings

Adicionar:

```text
refresh
startup
shortcut
runtime preference
providers enabled
environments enabled
```

---

# Fase 12 — Diagnostics

Criar:

```text
provider diagnostics
runtime diagnostics
```

---

# Fase 13 — Outros providers

Adicionar providers restantes individualmente.

Não bloquear release caso alguma integração seja classificada como C ou D.

---

# Fase 14 — CI & Build (sem assinatura)

Configurar build automatizado (ex: GitHub Actions) gerando:

```text
Windows .msi
Linux .AppImage + .deb
macOS .dmg
```

Sem assinatura de código nem notarização no MVP — usuário libera manualmente via SmartScreen/Gatekeeper. Assinatura e notarização ficam para uma fase de release pós-MVP, condicionada à obtenção de certificado Windows e conta Apple Developer Program.

---

# 48. Testes

Prioridades:

## Runtime

Testar:

```text
Windows command
Linux command
WSL command
timeout
command missing
```

---

## Provider detection

Testar:

```text
binary exists
binary missing
custom path
multiple runtimes
```

---

## Parsers

Criar fixtures:

```text
tests/fixtures/
```

Exemplo:

```text
claude-status.txt
codex-status.txt
gemini-status.txt
```

---

# 49. Erros

Nenhum erro deve derrubar aplicação.

Exemplo:

```text
Claude
ERROR

Codex
OK

Gemini
OK

Grok
Unavailable
```

---

# 50. Logging

Logs:

```text
INFO
WARN
ERROR
DEBUG
```

Exemplo:

```text
INFO runtime WSL Ubuntu detected

INFO claude found in WSL Ubuntu

INFO codex found in Windows

WARN grok not detected

ERROR gemini usage timeout
```

Nunca logar segredo.

---

# 51. Primeira entrega

Implementar inicialmente:

```text
Fases 0–5
```

Ou seja:

1. projeto Tauri;
2. overlay;
3. tray;
4. hotkey;
5. Runtime abstraction;
6. WindowsRuntime;
7. LinuxRuntime;
8. MacRuntime;
9. WslRuntime;
10. descoberta de distros;
11. Provider Registry;
12. detecção de binários;
13. cards usando usage fake.

Não implementar usage real ainda.

---

# 52. Critério da primeira entrega

No Windows, Linux, macOS e WSL:

Ao iniciar o Quotify:

```text
Tray icon aparece
```

Ao pressionar:

```text
Ctrl + Shift + U
```

overlay abre.

O Quotify deve conseguir mostrar algo semelhante a:

```text
Claude
WSL Ubuntu
Detected

Codex
Windows
Detected

Gemini
Windows
Detected

Codex
macOS
Detected

Grok
Not detected
```

Os valores de quota ainda podem ser fake.

---

# 53. Segunda entrega

Executar provider research.

Criar:

```text
docs/provider-research.md
```

Somente então implementar primeiro collector real.

---

# 54. Definição de pronto do MVP

O MVP estará pronto quando:

* Windows funcionar;
* Linux funcionar;
* macOS funcionar;
* WSL for detectado automaticamente;
* runtimes forem abstraídos;
* providers forem detectados em diferentes runtimes;
* overlay funcionar;
* tray funcionar;
* hotkey funcionar;
* refresh funcionar;
* pelo menos 2 providers tiverem usage real;
* múltiplos runtimes não gerarem crash;
* providers falhos não afetarem os demais;
* SQLite persistir snapshots;
* diagnostics existirem;
* custom executable funcionar;
* última informação válida permanecer visível durante erro.

---

# 55. Regras de arquitetura

Nunca implementar:

```text
ClaudeWindowsAdapter
ClaudeLinuxAdapter
ClaudeWslAdapter
```

Preferir:

```text
ClaudeAdapter
+
Runtime
```

---

Nunca fazer:

```text
if windows
else if linux
else if wsl
```

espalhado dentro de adapters.

Essa decisão pertence ao Runtime Layer.

---

Nunca assumir:

```text
PATH
HOME
shell
filesystem
```

do sistema host quando trabalhando com outro runtime.

---

# 56. Extensibilidade futura

A abstração de Runtime deve permitir futuramente:

```text
Runtime
├── Windows
├── Linux
├── WSL
├── Docker
├── Dev Container
└── SSH
```

Sem precisar reescrever:

```text
ClaudeAdapter
CodexAdapter
GeminiAdapter
GrokAdapter
```

---

# 57. Direção do produto

Quotify não deve virar inicialmente:

* IDE;
* launcher;
* gerenciador de prompts;
* orquestrador de agentes;
* cliente de chat.

Ele deve continuar sendo um:

**Agent Usage HUD.**

O valor está em oferecer uma visão imediata sobre disponibilidade dos agentes usados pelo desenvolvedor.

---

# 58. Primeira instrução ao implementar

Comece somente pelas fases:

```text
0
1
2
3
4
5
```

Não implemente ainda nenhuma integração real de quota.

Primeiro garanta que:

```text
Windows
Linux
macOS
WSL
RuntimeManager
CommandRunner
ProviderRegistry
Overlay
Tray
Hotkey
```

estejam arquiteturalmente sólidos.

Depois disso, faça o spike técnico de cada provider antes de escolher qualquer método de coleta.

Priorize arquitetura extensível e código simples.

Evite abstrações prematuras além das necessárias para suportar:

```text
Windows
Linux
macOS
WSL
```

O Quotify deve nascer pequeno, mas sem ficar preso ao sistema operacional do host.

---

# 59. Decisões de fechamento do plano (2026-09-17)

Registro das decisões tomadas para fechar as lacunas identificadas antes do início do desenvolvimento. Servem como referência para as fases e para futuras revisões do plano.

## macOS

Incluído no escopo do MVP. `MacRuntime` é implementado junto com Windows/Linux/WSL desde a Fase 2/3 — não é uma extensão pós-MVP.

## Wayland

MVP cobre X11/XWayland. Wayland nativo tem suporte limitado a always-on-top e hotkeys globais no Tauri 2; fica registrado como limitação conhecida, sem trabalho de mitigação dedicado no MVP.

## Runtime automático

Não há resolução automática real entre múltiplas instalações no MVP. Quando um provider tiver mais de uma instalação detectada, a escolha do runtime é manual (seção 35). "Automatic" só se aplica implicitamente quando existe exatamente uma instalação.

## Concorrência do refresh

`refreshAll()` (seção 26) executa providers/runtimes de forma sequencial no MVP. Paralelismo com limite de concorrência é otimização futura.

## Status visual para unidades não percentuais

Quando não houver `percentage` calculável (créditos, requests, tokens — seção 20), exibir o valor bruto em badge neutro, sem cores de warning/critical (seção 33).

## Migrations do SQLite

Adotar migrations versionadas simples desde a Fase 10, mesmo no MVP, para não travar a evolução do schema.

## CI, build e assinatura

Build automatizado (ex: GitHub Actions) gerando `.msi` / `.AppImage` + `.deb` / `.dmg` entra no MVP (Fase 14). Assinatura de código e notarização ficam fora do MVP — dependem de certificado Windows e conta Apple Developer Program — e são tratadas em uma fase de release posterior.

## Ordem de implementação dos Runtimes

Desenvolvimento inicial ocorre em Windows + WSL2. `WindowsRuntime` e `WslRuntime` são implementados e validados nativamente primeiro (Fase 2). `LinuxRuntime` e `MacRuntime` são implementados com a mesma interface e testados via unit tests desde o início, mas a validação end-to-end em máquina real fica pendente até haver acesso a Linux e macOS — CI (Fase 14) cobre a lacuna ao rodar builds/testes nesses SOs mesmo sem uma máquina de desenvolvimento local disponível.
