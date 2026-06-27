# Auditoria do Polymarket Bot

> Documento de referência gerado em **2026-06-27**. Snapshot do estado do código na branch `main` (último release `v0.6.35`, commit `109714e`) com as alterações não commitadas presentes na árvore de trabalho.

---

## 1. Resumo executivo

Projeto é um **workspace Cargo com 2 bots Rust independentes** + biblioteca compartilhada + pipeline de ML em Python + PostgreSQL. Apenas paper trading (research, sem dinheiro real).

| Aspecto | Estado |
|---|---|
| Arquitetura | Madura, bem separada em 3 crates + sidecar Python |
| Cobertura de testes | 154 testes (`#[test]`/`#[tokio::test]`) |
| **Build atual** | ✅ **CORRIGIDO em 2026-06-27** — antes quebrava com 17 erros (TUI WIP). Ver seção 8. |
| CI | Passa localmente: build + `cargo clippy --all-targets -Dwarnings` + fmt + 151 testes ok |
| Versão | `0.6.35` consistente nos dois Cargo.toml |

**Histórico:** o WIP do TUI (`trading-bot/src/tui.rs`) estava incompleto e impedia a compilação. Foi finalizado e unificado sobre a camada SQLx existente (detalhes na seção 8).

---

## 2. Estrutura do workspace

```
polymarket-bot/                  # raiz do workspace Cargo
├── common/        (polymarket-common, lib)   ~4.858 LOC
├── trading-bot/   (bin ML)                    ~8.222 LOC
├── copy-trading-bot/ (bin copy)               ~1.582 LOC
├── scripts/       (pipeline ML Python)        ~2.821 LOC
├── migrations/    (SQLx, 18 arquivos)
├── monitoring/    (Prometheus + Grafana)
├── adr/           (10 ADRs)
└── analysis/      (queries SQL de performance)
```

54 arquivos `.rs` no total. `edition = 2024` em todos os crates.

### Crates

- **`polymarket-common`** — tipos de domínio (`Bet`, `NewBet`, `BetContext`, `CopyRef`, `GammaMarket`, `PriceTick`), persistência Postgres (`storage/postgres.rs`, 58 KB — maior arquivo), feature engineering (`model/features.rs`, v5/29 features), Kelly (`pricing/kelly.rs`), notifier Telegram, métricas Prometheus, formatação.
- **`trading-bot`** — bot de sinais ML. Entrypoint `main.rs` orquestra múltiplos loops em `live.rs`. Scanner em `scanner/live.rs` (88 KB — núcleo do pipeline). Estratégias, bayesiano, calibração, backtest, modelo (xgb local + sidecar).
- **`copy-trading-bot`** — espelha trades de traders do leaderboard. Menor e mais simples.

---

## 3. Pipeline do trading-bot (como funciona)

Fluxo (ver README e `scanner/live.rs`):

1. **Fetch** mercados elegíveis da Gamma API (filtros: volume, expiry, preço).
2. **Feature engineering** — 29 features v5 (preço/momentum/RSI/volatilidade, temporais, 16 NLP do texto da pergunta).
3. **XGBoost ensemble** — local (Rust puro, travessia de árvore JSON em `model/xgb.rs`) **ou** sidecar Python (`model/sidecar.rs`).
4. **Bayesian anchoring** (`bayesian.rs`) — prediction ancorada no preço de mercado; LR amortecido por `LR^(confidence × LR_DAMPING)`.
5. **Filtros de sinal** (ADR 009) — bloqueia sports/esports e lado YES de XGBoost.
6. **Correlation check** (ADR 007) — LLM detecta apostas correlacionadas/mutuamente exclusivas (fail-open).
7. **Avaliação por estratégia** — cada perfil checa thresholds de edge/confiança independentemente.
8. **Kelly sizing** (`strategy.rs::evaluate`) — Kelly fracionário + escala de risco terminal + gates de min-bet.
9. **Placement** — aposta em papel gravada no DB com snapshot de features; notificação Telegram.

### Loops concorrentes (`live.rs::run_live`)

São spawnados via `tokio::spawn` e supervisionados por `tokio::select!`:

| Loop | Intervalo | Função |
|---|---|---|
| `housekeeping` | `SCAN_INTERVAL_MINS` (30) | resolve apostas, stop-loss, expiry exit, calibração, freshness do modelo |
| `bet_scan` | `BET_SCAN_INTERVAL_MINS` (10) | scoring de mercado + apostas |
| `command_loop` | poll 3s | comandos Telegram |
| `heartbeat` | `HEARTBEAT_INTERVAL_MINS` (60) | resumo periódico |
| `ws_loop` + `ws_refresh` | refresh 30min | WebSocket de preços (alerta em movimento 3%) |
| alert poller modelo | 60s | métrica de idade do modelo |
| `alert_loop` | event-driven | reavaliação XGBoost disparada por WS |

**Observação:** se qualquer loop crítico (housekeeping/bet_scan/etc.) sair, `tokio::select!` derruba o processo inteiro logando o erro — não há reinício automático do loop individual. Depende do restart do container (Docker).

### Estratégias (3 simultâneas, bankrolls isolados)

| Estratégia | Kelly | Min Edge | Min Conf | Max Sinais/dia | Min Bet |
|---|---|---|---|---|---|
| Aggressive | 50% | 5% | 40% | 10 | €5 |
| Balanced | 25% | 6% | 40% | 5 | €5 |
| Conservative | 15% | 8% | 50% | 3 | €15 |

Sinais XGBoost recebem thresholds reduzidos (edge × 0.5, conf × 0.7) — `strategy.rs:87`. Escala de risco terminal reduz tamanho para apostas > 3 dias até expiry (`strategy.rs:112`).

---

## 4. Camada de dados / migrations

- Persistência via **SQLx** (Postgres) em runtime normal.
- 18 migrations em `migrations/`. **Lacuna de numeração:** existem `001–005` e `008–020`, mas **`006` e `007` estão ausentes** dos arquivos. Provavelmente foram squashed/removidas — verificar se não há `_sqlx_migrations` esperando esses IDs em bancos antigos (risco de checksum mismatch em deploys legados).
- Migrations cobrem: init, calibração, estratégias, sinais rejeitados, usuários Telegram, prediction log, bet source/url/category, índices compostos, copy trading, features, correlação.

---

## 5. ML / Sidecar Python

- `scripts/serve_model.py` — FastAPI sidecar: `/predict`, `/predict_batch`, `/reload`, `/retrain`, `/retrain/status`, `/health`.
- `scripts/train_model.py` — treina ensemble (XGBoost + LightGBM + HistGBM + ExtraTrees + RF + meta-learner), exporta JSON pro Rust.
- `scripts/fetch_data.py` — busca mercados resolvidos + histórico de preço → `training_data.json`.
- Retrain a cada 24h (~3000 mercados resolvidos + apostas próprias com peso 3x).
- **Pin crítico:** `scikit-learn==1.8.0` — o `ensemble.joblib` é pickled com 1.8.0; 1.9.0 quebra unpickle (`ModuleNotFoundError: _loss`). Só subir junto com retrain. Documentado em `requirements.txt` e fixado no commit `44ceac6`.
- Artefatos do modelo (`/model/`) são **gitignored** — não estão no repo, gerados em runtime/deploy.

---

## 6. Infra / Operação

- **Docker Compose**: 6 serviços (postgres 17, model-server, bot, copy-trading-bot, prometheus, grafana:3000).
- **Métricas**: trading-bot em `:9000`, copy-trading-bot em `:9001`. Coletor de runtime tokio (`tokio_unstable`).
- **CI** (`.github/workflows/ci.yml`): `cargo fmt --check` → `cargo clippy --workspace --all-targets` → `cargo test --workspace`, com `RUSTFLAGS=--cfg tokio_unstable -Dwarnings`. **Warnings = erro.**
- **Deploy**: build de imagens → Hetzner VPS via SSH (workflow `deploy.yml`).
- **Release**: `workflow_dispatch` manual bumpa versão nos dois Cargo.toml, taga, cria release.

---

## 7. Estado das alterações não commitadas (WIP)

`git status` mostra:

```
 M Cargo.lock              (+601 linhas — novas deps)
 M docker-compose.yml      (expõe porta 5432 do postgres no host)
 M trading-bot/Cargo.toml  (+ratatui, +deadpool-postgres, +tokio-postgres)
 M trading-bot/src/main.rs (+modos "test" e "tui", +setup_database_pool)
?? trading-bot/src/tui.rs            (NOVO — TUI ratatui)
?? docker-compose.yml.backup         (lixo, remover)
?? trading-bot/Cargo.toml.backup     (lixo, remover)
```

Objetivo do WIP: adicionar um **TUI (ratatui)** que mostra sinais recentes (bets + rejected_signals) em tempo real, lendo do Postgres. Novos modos de CLI: `test` (bot em background + TUI) e `tui` (só TUI).

---

## 8. 🔴 Problemas / Riscos encontrados

### Críticos (quebram build/CI) — ✅ RESOLVIDOS em 2026-06-27

1. ✅ **`tui.rs` não compilava — 17 erros** (`cargo check -p trading-bot`). Causas e correções aplicadas:
   - **`crossterm` não era dependência direta** (8 erros `E0433`). ratatui 0.26 não re-exporta `crossterm`. **Fix:** adicionado `crossterm = "0.27"` ao Cargo.toml.
   - **Constantes de cor bare** `Green`/`Red`/`Yellow`/`White` sem `Color::` (6 erros `E0425`). **Fix:** prefixadas com `Color::`.
   - **`truncate_string` indefinida** (1 erro). **Fix:** implementada (trunca por chars, sufixo `…`).
   - **`BetContext`/`CopyRef` não impl `tokio_postgres::FromSql`** (2 erros `E0277`). **Fix:** TUI reescrito sobre SQLx — ver item 3.

2. ✅ **8 warnings (variáveis não usadas)** que `-Dwarnings` transformava em erro. **Fix:** TUI reescrito sem código morto; `clippy --all-targets -Dwarnings` passa limpo.

### Altos

3. ✅ **Duas camadas de DB divergentes.** O TUI usava `deadpool-postgres`/`tokio-postgres` (pool paralelo ao SQLx do resto). **Fix:** `tui.rs` reescrito sobre o `sqlx::PgPool` existente; deps `deadpool-postgres` e `tokio-postgres` removidas.

4. ✅ **`setup_database_pool` ignorava `DATABASE_URL`** — hardcodava `localhost`/`bot`/`bot`/`NoTls`. **Fix:** agora `PgPool::connect(&cfg.database_url)`. Funciona em Docker e com credenciais reais.

5. ⚠️ **PENDENTE — `docker-compose.yml` expõe `5432:5432` no host.** Conveniente para dev local, mas expõe o Postgres (credenciais `bot`/`bot`) — não deve ir pra produção sem restrição. Ver branch `fix/restrict-internal-ports`.

### Médios / Higiene

6. ✅ **Arquivos `.backup`** (`docker-compose.yml.backup`, `trading-bot/Cargo.toml.backup`) — removidos.

7. ✅ **TUI não entrava em raw mode / alternate screen.** **Fix:** `run_tui` agora chama `enable_raw_mode`/`EnterAlternateScreen` e restaura (`disable_raw_mode`/`LeaveAlternateScreen`/`show_cursor`) sempre no fim, mesmo em erro.

8. **Lacuna de migrations 006/007** — confirmar que não causa checksum mismatch em bancos existentes.

9. **68 ocorrências de `.unwrap()`/`.expect()`/`panic!`** em código não-teste. Maioria provavelmente em paths de init/config (aceitável), mas vale auditar os que ficam em loops de runtime — um panic dentro de um loop spawnado pode derrubar o processo.

10. **Sem reinício de loop individual** — qualquer loop crítico que retorne encerra o processo (depende de restart externo do container).

### Positivos

- 154 testes, boa cobertura de parsing/estratégia/calibração.
- Arquitetura limpa, ADRs documentando decisões.
- Shutdown gracioso (SIGTERM/SIGINT) com notificação.
- Retry de conexão ao DB (10 tentativas) no path principal (`live.rs:77`).
- Pin de sklearn bem documentado.

---

## 9. ADRs

| # | Título | Status |
|---|---|---|
| 001 | Baseline model audit | active |
| 002 | Fix training/live inconsistencies | complete |
| 003 | Fresh data fetch | complete |
| 004 | Online learning | draft |
| 005 | Feature improvements | active |
| 006 | Remove dead features (15→12) | active |
| 007 | Portfolio correlation check (LLM) | draft |
| 008 | Workspace split (3 crates) | accepted |
| 009 | Profitability overhaul (signal filters) | active |
| 010 | Remove dataset scripts | active |

Branches remotas abertas sugerem trabalho em andamento: `feat/signal-filters`, `feat/llm-correlation-check`, `feat/separate-bet-scan-cycle`, `model/online-learning`, `model/feature-improvements`, `fix/restrict-internal-ports`, `fix/sidecar-schema-mismatch`.

---

## 10. Recomendações priorizadas

1. ✅ **Destravar o build** — feito (TUI corrigido).
2. ✅ **Unificar acesso ao DB** — feito (TUI sobre `sqlx::PgPool`).
3. ✅ **Corrigir `setup_database_pool`** — feito (lê `DATABASE_URL`).
4. ✅ **Completar o TUI** — feito (raw mode, alternate screen, restauração no exit, sem código morto).
5. ✅ **Remover os `.backup`** — feito. (Pendente opcional: adicionar `*.backup` ao `.gitignore`.)
6. ⚠️ **Revisar exposição da porta 5432** antes de qualquer deploy — pendente.
7. Rodar `cargo fmt --all` + `cargo clippy --all-targets` localmente antes do commit (espelha o CI).

### Pendências remanescentes (não bloqueiam build)

- Exposição da porta 5432 no `docker-compose.yml` (seção 8, item 5).
- Lacuna de migrations 006/007 (seção 4 / 8 item 8).
- Auditar `.unwrap()`/`panic!` em loops de runtime (seção 8 item 9).
- Sem reinício de loop individual (seção 8 item 10).

---

*Para regenerar/atualizar esta auditoria: revisar `git status`, rodar `cargo check --workspace`, e conferir as seções 7 e 8.*
