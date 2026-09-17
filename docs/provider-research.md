# Provider Research

Spike da Fase 6 (docs/PLAN.md seção 24). Ambiente de teste: WSL2 Ubuntu, dentro do mesmo host Windows+WSL2 usado no desenvolvimento. Windows nativo e macOS **não foram testados nesta rodada** — os achados marcados "não verificado" precisam de confirmação manual nesses SOs antes de qualquer adapter real ser implementado (Fase 7+).

Princípio seguido (seção 25): nada aqui deve virar dado inventado. Onde não há confirmação real, está marcado como tal.

---

# Claude Code

CLI encontrada:
sim — testado nesta WSL (`claude --version` → `2.1.247 (Claude Code)`)

Ambientes testados:
WSL (Ubuntu). Windows e macOS não testados nesta rodada — mesmo binário distribuído via npm, comportamento esperado igual, mas não confirmado.

Método de autenticação:
OAuth via conta Anthropic (`claude auth login`) ou API key. Credenciais ficam em `~/.claude/.credentials.json` — não inspecionado neste research (nunca ler/logar credenciais, seção 30).

Método de usage:
Nenhum comando `usage`/`quota` dedicado apareceu em `claude --help`. Existe `claude auth status --json` ("Show authentication status") — candidato mais próximo, mas **não confirmado** se retorna números de uso/limite ou só dados de conta/sessão. Dentro de uma sessão interativa existe `/cost`, mas é um slash command da TUI, não invocável de forma não-interativa por um collector em background. Não há endpoint documentado publicamente que o CLI exponha de forma limpa para polling.

Reset disponível:
Não identificado via CLI. O dashboard web (console.anthropic.com / claude.ai) tem indicadores de uso, mas isso é scraping de UI (último recurso, seção 22).

Método escolhido:
Investigar `claude auth status --json` como primeira tentativa (CLI oficial, seção 22 item 2) antes de qualquer outra estratégia — mas isso ainda precisa ser rodado e a saída inspecionada manualmente para confirmar se carrega números de quota.

Fallback:
Nenhum identificado com confiança suficiente ainda. Não inventar `percentage` (seção 20) enquanto isso não for confirmado.

Confiabilidade:
C — CLI existe, responde bem, subcomando de auth promissor, mas nenhuma fonte de uso/quota confirmada até agora.

---

# Codex (OpenAI)

CLI encontrada:
sim — testado nesta WSL (`codex --version` → `codex-cli 0.154.0`)

Ambientes testados:
WSL (Ubuntu). Windows e macOS não testados nesta rodada.

Método de autenticação:
`codex login` / `codex logout`. Credenciais em `~/.codex/auth.json` — não inspecionado.

Método de usage:
Nenhum comando `usage`/`quota` em `codex --help`. Existe `codex doctor --json` ("Emit a redacted machine-readable report") — candidato a investigar, mas não confirmado se inclui uso/limite ou só diagnóstico de instalação/auth/runtime.

Reset disponível:
Não identificado.

Método escolhido:
Investigar `codex doctor --json` manualmente antes de decidir. Sem isso, não há método confiável ainda.

Fallback:
Nenhum identificado.

Confiabilidade:
D — nenhum sinal claro de exposição de quota/usage via CLI ainda; precisa de investigação manual adicional antes de subir para C ou B.

---

# Gemini CLI (Google)

CLI encontrada:
não testado neste ambiente — binário `gemini` não está instalado na máquina usada para este research.

Ambientes testados:
nenhum.

Método de autenticação:
Não verificado. Expectativa (não confirmada): OAuth via conta Google e/ou API key do Google AI Studio.

Método de usage:
Não verificado.

Reset disponível:
Não verificado.

Método escolhido:
Nenhum ainda — primeiro passo real é instalar o `gemini` CLI em uma máquina de teste e repetir os mesmos passos usados aqui para Claude/Codex (`--help`, `--version`, procurar subcomandos de auth/status/doctor).

Fallback:
Nenhum.

Confiabilidade:
D — sem qualquer verificação direta.

---

# Grok (xAI)

CLI encontrada:
não testado neste ambiente — nem sequer confirmado se existe uma CLI oficial amplamente distribuída da xAI chamada `grok` no mesmo padrão das outras três.

Ambientes testados:
nenhum.

Método de autenticação:
Desconhecido.

Método de usage:
Desconhecido.

Reset disponível:
Desconhecido.

Método escolhido:
Nenhum — o passo zero aqui é confirmar se existe CLI oficial. Se não existir, Grok pode precisar ficar classificado D permanentemente (ou fora do escopo dos "2 providers reais" do MVP, seção 9/54) até que a xAI publique uma.

Fallback:
Nenhum.

Confiabilidade:
D.

---

# Resumo e próximo passo

| Provider | CLI encontrada | Confiabilidade |
|---|---|---|
| Claude Code | sim | C |
| Codex | sim | D |
| Gemini | não verificado | D |
| Grok | não verificado | D |

Nenhum provider está pronto para virar adapter real ainda — todos precisam de uma rodada de verificação manual (rodar os comandos candidatos de verdade e inspecionar a saída) antes da Fase 7/8. Claude Code é o candidato mais próximo de A/B, mas depende de confirmar se `claude auth status --json` carrega dado de uso real.
