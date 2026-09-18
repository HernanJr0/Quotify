# Provider Research

Pesquisa iniciada na Fase 6 e atualizada durante as implementações reais das Fases 8 e 9 (docs/PLAN.md seção 24). Ambiente principal: WSL2 Ubuntu no mesmo host Windows+WSL2 usado no desenvolvimento. As seções de cada provider registram separadamente os ambientes realmente validados.

Princípio seguido (seção 25): nada aqui deve virar dado inventado. Onde não há confirmação real, está marcado como tal.

---

# Claude Code

CLI encontrada:
sim — testado na WSL e no Windows nativo (`claude --version` → `2.1.247 (Claude Code)` durante a pesquisa)

Ambientes testados:
WSL (Ubuntu) e Windows nativo. O control request foi executado de ponta a ponta no executável Windows. macOS ainda não foi testado.

Método de autenticação:
OAuth via conta Anthropic (`claude auth login`) ou API key, administrado integralmente pelo próprio Claude Code. O adapter do Quotify não lê credenciais.

Método de usage:
O `claude auth status --json` informa autenticação, não a cota. Dentro da TUI, `/usage` mostra a cota da assinatura. Desde Claude Code 2.1.x, a [statusline oficial](https://code.claude.com/docs/en/statusline) recebe após a primeira resposta os campos `rate_limits.five_hour` e `rate_limits.seven_day`, com `used_percentage` e `resets_at`. Esse é o caminho público mais estável, mas só existe enquanto uma sessão interativa está ativa.

Versões atuais também aceitam um control request experimental `get_usage` quando o CLI é iniciado com entrada/saída `stream-json`. A resposta estruturada contém as janelas em `rate_limits.limits[]`, incluindo `kind`, `percent`, `resets_at` e escopo por modelo. O próprio Claude Code cuida da autenticação e eventual refresh; a consulta não chama o modelo (`total_cost_usd: 0`, `model_usage: {}`). É o mecanismo usado pelo Agent SDK, embora a API ainda esteja explicitamente marcada como experimental.

O teste no Windows confirmou a resposta estruturada e custo zero. Na conta sem assinatura Pro/Max, a resposta trouxe `rate_limits_available: true`, mas `rate_limits: null`; portanto, disponibilidade do protocolo não significa que exista uma cota de assinatura ativa.

Também foi investigado o endpoint interno `GET https://api.anthropic.com/api/oauth/usage`, usado por alguns monitores da comunidade. Ele funciona com o OAuth mantido pelo CLI, mas não faz parte da API pública e o teste direto sofreu `429`. Além da fragilidade técnica, o acesso automatizado direto é difícil de conciliar com a cláusula 3.7 dos [Consumer Terms da Anthropic](https://www.anthropic.com/legal/consumer-terms). Por isso ele foi removido do Quotify.

Reset disponível:
Sim. Tanto o payload oficial de statusline quanto o endpoint OAuth retornam o instante exato de reset das janelas de 5 horas e semanal.

Método escolhido:
O adapter pergunta exclusivamente ao próprio CLI via `get_usage`, desabilitando MCPs e hooks apenas para o processo de probe. Isso evita tocar nas credenciais e permite que o CLI renove tokens expirados. A janela de 5 horas é a métrica primária; a semanal é fallback quando a primeira não vier. O resultado fica em cache por cinco minutos e o último snapshot pode ser exibido como cache se uma atualização falhar.

Fallback:
Nenhum fallback automático. A statusline permanece como alternativa futura, mediante consentimento explícito para instalar um bridge que grave o último `rate_limits` em cache local. Scraping da TUI via PTY (`/usage`) também existe em outros projetos, mas é mais frágil.

Confiabilidade:
B — o CLI é confiável e foi validado nos dois ambientes, mas `get_usage` ainda está explicitamente marcado como experimental e pode mudar de schema.

---

# Codex (OpenAI)

CLI encontrada:
sim — testado nesta WSL (`codex --version` → `codex-cli 0.155.0`)

Ambientes testados:
WSL (Ubuntu). Windows e macOS não testados nesta rodada.

Método de autenticação:
`codex login` / `codex logout`. O `app-server` usa e renova a sessão administrada pelo próprio Codex; o Quotify não lê credenciais.

Método de usage:
`codex doctor --json` foi descartado porque contém somente diagnóstico. A [documentação oficial do Codex App Server](https://developers.openai.com/codex/app-server) define o transporte stdio JSONL, o handshake `initialize`/`initialized` e o método `account/rateLimits/read`. O resultado contém a visão compatível `rateLimits` e, quando aplicável, `rateLimitsByLimitId`; cada janela informa `usedPercent`, `windowDurationMins` e `resetsAt`.

O fluxo foi validado de ponta a ponta com a conta ChatGPT autenticada nesta WSL. A resposta real trouxe uma janela primária de 300 minutos e uma secundária de 10.080 minutos, ambas com percentual e reset, sem iniciar conversa ou turno de modelo.

Reset disponível:
Sim, como timestamp Unix em segundos (`resetsAt`).

Método escolhido:
Iniciar `codex app-server` (stdio já é o transporte padrão; versões nativas mais antigas rejeitam o alias `--stdio`), identificar o Quotify no handshake e chamar `account/rateLimits/read`. O processo fica aberto somente até a resposta de id `2`, quando é encerrado pelo runtime. A janela primária é exibida; a secundária serve como fallback se a primária estiver ausente. Timeout de 15 segundos, cache de cinco minutos e último snapshot como fallback de falha.

Fallback:
A visão `rateLimits` é usada quando o mapa `rateLimitsByLimitId.codex` não vier. Não há acesso direto a tokens nem endpoint privado.

Confiabilidade:
A — mecanismo oficial, documentado e validado ao vivo no ambiente local.

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
| Claude Code | sim | B |
| Codex | sim | A |
| Gemini | não verificado | D |
| Grok | não verificado | D |

Claude Code e Codex possuem adapters reais. O Claude usa somente o CLI experimental e fica indisponível quando a conta não possui cota de assinatura; o Codex usa o `app-server` documentado e já retornou as janelas reais da conta. Gemini e Grok continuam sem verificação direta suficiente.
