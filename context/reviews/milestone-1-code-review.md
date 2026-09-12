# Code Review — Milestone 1

- **Data:** 2026-09-12
- **Escopo:** implementação dos Milestones 0 e 1
- **Status:** correções de código implementadas; evidências de compositor ainda pendentes
- **Gates relacionados:** G00, G01 e G02

## Objetivo

Corrigir os problemas encontrados no primeiro review antes de considerar o
Milestone 1 concluído ou iniciar o Milestone 2.

## Resumo

`cargo check` e `cargo test` passam, mas o projeto ainda não prova a integração
entre GPUI e layer-shell. A CI também falha nas etapas de formatação e Clippy.

| Prioridade | Achado | Gate afetado | Status |
| --- | --- | --- | --- |
| P1 | Layer-shell não está integrado ao GPUI | G02 | Implementado; validar no Hyprland |
| P1 | CI falha em `cargo fmt` e Clippy | G00 | Corrigido e validado |
| P1 | Falhas de abertura do GPUI retornam sucesso | G01 | Corrigido e validado |
| P2 | Transparência não é demonstrada | G01 | Implementado; falta screenshot |
| P2 | Lifecycle de buffers Wayland é incompleto | G01, G02 | Corrigido e testado |
| P2 | Input e seleção de output não são validados | G02 | Implementado; validar no Hyprland |
| P2 | Fallback de `xkbcommon-x11` não é portável | G00, G01 | Corrigido |

## Resultado da implementação

- O frontend usa `WindowKind::LayerShell` do GPUI em uma revisão Wayland-only
  fixada, sem janela XDG auxiliar.
- `SurfaceSpec` é convertido para layer, anchors, exclusive zone e
  keyboard-interactivity do GPUI.
- O elemento raiz do frontend é transparente; o conteúdo visível usa uma
  região com alpha.
- Falhas do backend GPUI/Wayland são convertidas em `PlatformError` e levam a
  exit code diferente de zero.
- O PoC usa os tipos de `shell-platform`, enumera `wl_output` por nome e limita
  o bind à versão anunciada. Também registra capabilities de `wl_seat`, pointer
  e keyboard.
- Buffers SHM são reutilizados quando compatíveis e só são destruídos depois de
  `wl_buffer.release` ou durante shutdown quando já estão disponíveis.
- O workaround que criava symlink para `libxkbcommon-x11.so` foi removido. A
  CI instala explicitamente as dependências nativas Wayland.

### Validação automatizada

Os quatro gates definidos em R02 passam localmente:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace --all-targets
cargo test --workspace
```

Também foi verificado que `cargo run --bin shell-app` termina com sucesso e que
`cargo run --bin shell-app -- --gpui-viability` termina com código 1 e uma
mensagem de inicialização Wayland quando não há compositor disponível.

### Limitações de evidência

O ambiente de execução desta implementação não possui uma sessão Wayland ativa.
Portanto, ainda precisam ser capturados no Hyprland os screenshots e logs
listados na seção de evidências. A API `LayerShellOptions` da revisão fixada do
GPUI não expõe um campo de output; a seleção por nome está disponível no PoC
Wayland, enquanto o adapter GPUI usa o output padrão do compositor até existir
um contrato de display no upstream.

## Plano de correção

### R01 — Integrar layer-shell ao frontend GPUI

**Problema**

O frontend GPUI cria uma janela XDG comum. O PoC de layer-shell é um processo
separado que apresenta apenas um buffer SHM transparente. Além disso, o PoC
duplica os enums de layer e teclado em vez de consumir `SurfaceSpec`.

**Ações**

1. Confirmar uma versão ou revisão do GPUI que exponha layer-shell de forma
   utilizável, ou implementar uma extensão pequena e isolada conforme ADR-004.
2. Fazer o adapter de plataforma consumir `SurfaceSpec` diretamente.
3. Remover `LayerMode` e `KeyboardMode` duplicados do PoC.
4. Renderizar conteúdo GPUI dentro de uma surface layer-shell real.
5. Se isso exigir um fork amplo ou integração instável, reabrir ADR-001/D002
   antes de prosseguir.

**Critérios de aceite**

- Uma surface GPUI é reconhecida pelo Hyprland como layer-shell.
- `Top` e `Overlay` funcionam com conteúdo GPUI visível.
- Anchors, exclusive zone e keyboard interactivity vêm de `SurfaceSpec`.
- O frontend não depende de uma janela XDG auxiliar.

### R02 — Restaurar a CI

**Problema**

`cargo fmt --check` encontra diferenças e Clippy rejeita
`LinuxServices::default()` por se tratar de uma unit struct.

**Ações**

1. Executar `cargo fmt --all`.
2. Substituir `LinuxServices::default()` por `LinuxServices`, ou transformar o
   tipo em uma estrutura com estado real quando sua inicialização for usada.
3. Executar Clippy para todos os targets do workspace.
4. Alinhar os comandos da CI com os comandos locais completos.

**Critérios de aceite**

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace --all-targets
cargo test --workspace
```

Todos os comandos devem terminar com exit code zero.

### R03 — Propagar falhas de inicialização do GPUI

**Problema**

Quando `open_window` falha, o frontend apenas escreve no stderr e encerra o
event loop. O processo termina com sucesso e pode produzir uma evidência falsa
para G01.

**Ações**

1. Fazer `run_viability_app` expor um resultado observável ao composition root.
2. Converter falhas de criação da janela em `PlatformError`.
3. Encerrar o executável com código diferente de zero quando G01 não puder ser
   iniciado.
4. Adicionar um teste para o caminho de erro sem depender de uma sessão Wayland.

**Critérios de aceite**

- Falha de conexão ou criação de janela produz exit code diferente de zero.
- O erro inclui contexto suficiente para distinguir Wayland, renderer e janela.
- O caminho de sucesso continua encerrando de forma limpa.

### R04 — Demonstrar transparência real

**Problema**

A janela pede background transparente, mas o elemento raiz cobre toda a área
com uma cor RGB opaca.

**Ações**

1. Manter o elemento raiz transparente.
2. Renderizar conteúdo visível em uma região menor ou usar uma cor com alpha.
3. Capturar uma evidência em que seja possível ver uma janela atrás da área
   transparente.

**Critérios de aceite**

- O compositor mostra conteúdo externo através da região transparente.
- O conteúdo customizado permanece legível.
- A evidência é armazenada em `context/evidences/`.

### R05 — Corrigir o lifecycle dos buffers Wayland

**Problema**

Cada evento `Configure` cria um novo `wl_buffer`. O buffer anterior é
substituído e o evento `wl_buffer.release` é ignorado, impedindo a liberação
segura dos objetos e da memória associada.

**Ações**

1. Associar estado próprio a cada `wl_buffer` criado.
2. Manter buffers em uso até o compositor emitir `release`.
3. Destruir buffers liberados explicitamente.
4. Reutilizar buffers compatíveis ou implementar uma pequena cadeia de buffers.
5. Destruir layer surface, surface e demais proxies durante shutdown normal.

**Critérios de aceite**

- Reconfigurações repetidas não aumentam indefinidamente objetos ou memória.
- Nenhum buffer é reutilizado ou destruído enquanto estiver em uso.
- O PoC encerra sem protocol errors.
- Um teste unitário cobre o estado do ciclo `created -> attached -> released`.

### R06 — Validar teclado, pointer e output específico

**Problema**

O PoC altera flags de interatividade, mas não recebe eventos de teclado ou
pointer. A seleção de output exige um número efêmero do registry e faz bind
direto na versão 4.

**Ações**

1. Registrar `wl_seat` e tratar mudanças de capabilities.
2. Registrar e exibir eventos básicos de pointer e teclado.
3. Demonstrar que uma input region vazia não recebe pointer.
4. Enumerar outputs, limitar a versão de bind à anunciada e selecionar por nome
   estável sempre que disponível.
5. Rejeitar argumentos inválidos antes de enviar requests ao compositor.

**Critérios de aceite**

- `KeyboardMode::None` não captura teclado.
- `OnDemand` ou `Exclusive` recebe uma tecla de teste e pode liberar o foco.
- O modo click-through permite interação com a janela abaixo.
- O usuário escolhe um monitor pelo nome e a surface aparece no monitor correto.
- Um output inexistente produz erro local, sem protocol error.

### R07 — Remover o fallback não portável de `xkbcommon-x11`

**Problema**

O build script procura bibliotecas em caminhos Debian/Ubuntu e cria um symlink
local para uma biblioteca versionada. Isso não declara corretamente a
dependência e não funciona de forma consistente em outras distribuições.

**Ações**

1. Remover o symlink criado pelo build script.
2. Verificar se uma versão/revisão do GPUI corrige a ativação indevida da
   feature X11 no build Wayland-only.
3. Se a dependência continuar obrigatória, documentar os pacotes nativos por
   distribuição e instalá-los explicitamente na CI.
4. Fazer o build falhar cedo com uma mensagem clara quando uma dependência
   obrigatória estiver ausente.

**Critérios de aceite**

- O projeto não depende de caminhos absolutos de uma distribuição.
- O build Wayland-only não ativa X11 quando isso puder ser evitado.
- A CI instala e valida todas as dependências nativas declaradas.
- Uma instalação limpa possui instruções reproduzíveis de build.

## Ordem recomendada

1. R02 — restaurar feedback confiável da CI.
2. R03 e R04 — tornar G01 verificável.
3. R05 e R06 — estabilizar o PoC Wayland.
4. R01 — concluir a integração arquitetural de layer-shell com GPUI.
5. R07 — consolidar a política de dependências após escolher a revisão do GPUI.

## Evidências necessárias antes de fechar o review

- Saída dos quatro comandos de validação da CI.
- Screenshot da transparência GPUI.
- Logs de input de teclado e pointer.
- Execução de `Top` e `Overlay`.
- Execução com e sem exclusive zone.
- Execução em output escolhido por nome.
- Teste com janela tiled, floating e fullscreen.
- Teste de lock/unlock com `hyprlock` sem crash.
- Registro das limitações conhecidas do backend GPUI selecionado.

## Condição de encerramento

Este review pode ser marcado como concluído somente quando todos os itens P1 e
P2 estiverem corrigidos ou quando uma decisão arquitetural registrar
explicitamente por que um item foi adiado. G01 e G02 devem permanecer abertos
até que suas evidências estejam anexadas.
