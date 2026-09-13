# Code Review — Milestones 2 e 3

- **Data:** 2026-09-12
- **Escopo:** `shell-platform` (topologia e PoC M2), `shell-hyprland`, modelos/ports do core e integração em `shell-app`.
- **Base:** estado atual do workspace. `HEAD` contém apenas o commit inicial; a implementação está em arquivos não rastreados, portanto não existe um diff de commits que isole M2 e M3.
- **Resultado:** 11 achados — 3 P1 e 8 P2. M2 e M3 ainda não devem ser considerados concluídos.
- **Alterações nesta revisão:** somente este relatório; nenhuma correção aplicada ao código de produção.

## Achados

### R01 — [P1] Converter resolução física para geometria lógica, incluindo rotação

**Local:** [translate.rs:120](../../crates/shell-hyprland/src/translate.rs#L120), linhas 120–126. **Gates:** G04/G05.

`monitor_snapshot` coloca `width` e `height` do IPC diretamente em um `Rect`, cujo contrato é usar coordenadas lógicas. Esses campos vêm da resolução física no [código do Hyprland](https://github.com/hyprwm/Hyprland/blob/v0.54.0/src/debug/HyprCtl.cpp#L186). O adapter também ignora `transform`.

**Reprodução confirmada:** um monitor de 2560×1440, escala 1,25, produz `Size(2560, 1440)` em vez de `Size(2048, 1152)`. Com 1920×1080 e rotação de 90°, produz 1920×1080 em vez de 1080×1920. O `OutputRegistry` usa essas dimensões para posicionar notch/panel e converter pontos; consequentemente os limites e a centralização ficam incorretos.

**Correção sugerida:** normalizar os eixos conforme `transform`, aplicar a escala e manter origem e tamanho no mesmo espaço lógico. Testar os valores esperados, além do round-trip de pontos já existente.

### R02 — [P1] Aplicar o monitor focado do snapshot ao registro de outputs

**Local:** [shell-app/src/lib.rs:145](../../crates/shell-app/src/lib.rs#L145), linhas 145–147. **Gates:** G04/G05.

`apply_compositor_snapshot` reconcilia os monitores, mas não aplica `snapshot.focused_output`. O registro escolhe o menor ID no primeiro snapshot e preserva esse foco enquanto o output existir. Assim, um evento de foco atualiza `compositor_state`, mas mantém um monitor diferente em `output_registry`.

**Reprodução confirmada:** snapshot com outputs 1 e 2 e foco no 2 deixa `output_registry.focused_output()` como `Some(1)`. Consumidores do registro selecionam a tela errada para surfaces e overlays.

**Correção sugerida:** aplicar `set_focused_output` e devolver sua transição junto às demais, tratando também `None`. Evitar publicar transições intermediárias contraditórias e testar a troca de foco sem alteração de geometria.

### R03 — [P1] Restringir input antes de mapear as surfaces transparentes de tela inteira

**Local:** [surface-topology-poc.rs:228](../../crates/shell-platform/src/bin/surface-topology-poc.rs#L228), linhas 228–233. **Gate:** G03.

No modo padrão, `click_through=false`, nenhuma input region é configurada. O protocolo Wayland define a região inicial como infinita, limitada ao tamanho da surface; transparência do buffer não altera essa regra. Tanto a surface `Unified` quanto `Overlay` em `Independent` cobrem a tela, estão em `Layer::Overlay` e recebem buffers completamente transparentes. Elas passam a interceptar os cliques na área coberta sem sequer haver um `wl_pointer` para processá-los.

`--click-through` torna todas as surfaces vazias para input, mas também elimina qualquer interação do panel/notch. Portanto o experimento não representa o comportamento necessário para comparar as topologias.

**Validação:** inspeção do fluxo de criação e do XML `wl_surface.set_input_region` da dependência instalada; não foi aberto um overlay bloqueador na sessão do usuário.

**Correção sugerida:** configurar hitboxes das regiões interativas e deixar o overlay ocioso sem input; expandir a região apenas durante a interação modal. Instrumentar pointer/keyboard e clique fora para medir os modos de G03.

### R04 — [P2] Preservar o registro quando um snapshot é rejeitado

**Local:** [topology.rs:221](../../crates/shell-platform/src/topology.rs#L221), linhas 221–237. **Gate:** G04.

`reconcile` modifica `self.outputs` durante a mesma iteração que valida os dados. Se uma entrada posterior tiver escala inválida ou ID duplicado, a função retorna `Err`, descartando as transições já produzidas, mas conserva as mutações anteriores.

**Reprodução confirmada:** partindo somente do output 1, aplicar `[output 1, output 2 válido, output 3 com escala 0]` retorna erro e deixa o output 2 registrado. No próximo snapshot válido, ele já existe e não produz `Added`; o consumidor que depende dessa transição pode nunca criar suas surfaces. Atualizações de geometria sofrem o mesmo problema.

**Correção sugerida:** validar o conjunto antes de modificar o registro ou reconciliar em um estado provisório e efetivar a operação apenas quando ela terminar com sucesso. Cobrir também duplicatas após entradas válidas.

### R05 — [P2] Manter IDs válidos de workspaces nomeados e especiais

**Local:** [translate.rs:80](../../crates/shell-hyprland/src/translate.rs#L80), linhas 80–86; também linhas 143–149. **Gate:** G05.

Os filtros `id >= 0` descartam IDs negativos que são válidos no Hyprland. Workspaces nomeados recebem IDs a partir de -1337, conforme a [alocação upstream](https://github.com/hyprwm/Hyprland/blob/v0.54.0/src/Compositor.cpp#L1445); os especiais também usam IDs negativos. `parse_workspaces` preserva esses IDs, mas a leitura do foco e a associação das janelas os convertem para `None`.

**Reprodução confirmada:** `activeworkspace` com ID -1337 perde o foco; uma janela com `workspace.id=-99` perde sua associação, embora esse workspace possa existir na lista traduzida.

**Correção sugerida:** distinguir sentinelas inválidas dos IDs válidos do protocolo, mantendo a identidade negativa no domínio. Testar lista, foco e associação da mesma entidade em conjunto.

### R06 — [P2] Resolver IDs negativos antes de enviar o comando de foco

**Local:** [shell-hyprland/src/lib.rs:89](../../crates/shell-hyprland/src/lib.rs#L89), linhas 89–91. **Gate:** G05.

`focus_workspace(WorkspaceId(-1337))` envia `dispatch workspace -1337`. Nesse dispatcher, o sinal negativo representa deslocamento relativo, e não o ID interno do workspace nomeado. Assim, selecionar um workspace retornado por `workspaces()` pode focar outro workspace e ainda receber `ok`. A distinção entre seletor por nome e deslocamento está na [documentação dos dispatchers](https://wiki.hypr.land/0.54.0/Configuring/Dispatchers/#workspaces).

**Validação:** formato enviado confirmado por leitura direta; interpretação do argumento confirmada no [parser upstream](https://github.com/hyprwm/Hyprland/blob/v0.54.0/src/helpers/MiscFunctions.cpp#L404). Nenhum comando de foco foi enviado ao compositor real.

**Correção sugerida:** resolver o ID para o seletor `name:...` ou `special:...` apropriado dentro do adapter, com erro explícito se a entidade desaparecer. Corrigir apenas os filtros de R05 não resolve este caminho de comando.

### R07 — [P2] Marcar a janela ativa como focada

**Local:** [translate.rs:208](../../crates/shell-hyprland/src/translate.rs#L208). **Gate:** G05.

`active_window` chama `parse_window` com `focused=None`. Como o parser define `focused` pela igualdade com `Some(id)`, toda janela retornada por `focused_window()` tem `focused=false`. A mesma janela na lista de `snapshot.windows` pode ter `focused=true`, expondo dois valores contraditórios para a mesma entidade.

**Reprodução confirmada:** payload válido de `activewindow` com endereço `0xabc` retorna uma janela cujo campo `focused` é falso.

**Correção sugerida:** marcar explicitamente o resultado de `active_window` como focado, preservando `{}` como `None`, e verificar consistência com a lista de janelas.

### R08 — [P2] Diferenciar maximização de fullscreen

**Local:** [translate.rs:241](../../crates/shell-hyprland/src/translate.rs#L241), linhas 241–245. **Gate:** G05.

A conversão `fullscreen != 0` interpreta o modo 1, maximizado, como fullscreen. O protocolo distingue 1 (maximizado) de 2/3 (fullscreen), conforme a [semântica de fullscreenstate](https://wiki.hypr.land/0.54.0/Configuring/Dispatchers/#fullscreenstate). Uma janela maximizada mantém a área de trabalho e suas margens, mas o adapter publica `Window.fullscreen=true` e pode publicar `FullscreenState::Active`.

**Reprodução confirmada:** payload com `fullscreen: 1` retorna `fullscreen=true`.

**Correção sugerida:** traduzir o estado interno conforme os valores do protocolo. Testar separadamente 0, 1, 2 e 3; o teste atual usa 1 como exemplo positivo de fullscreen e também precisa ser corrigido.

### R09 — [P2] Processar os eventos que invalidam título e configuração dos monitores

**Local:** [events.rs:41](../../crates/shell-hyprland/src/events.rs#L41), linhas 41–47. **Gates:** G04/G05.

`translate_event` ignora tanto `windowtitle`/`windowtitlev2` quanto `configreloaded`. Títulos de janelas podem mudar sem mudança de foco — por exemplo, ao trocar uma aba do navegador — e a lista de monitores pode mudar de escala, posição ou transformação sem add/remove. Como não há atualização periódica, essas mudanças permanecem invisíveis até chegar outro evento reconhecido. Os eventos estão na [lista oficial de IPC](https://wiki.hypr.land/IPC/#events-list).

**Reprodução confirmada para título:** um socket temporário envia `windowtitlev2` seguido de `workspacev2`; `next_event()` ignora o primeiro e retorna somente o evento `Workspace`. O evento de reload também cai no ramo `_ => None`, confirmado por inspeção.

**Correção sugerida:** incluir as invalidações de título e reload nas categorias adequadas e testar mudança sem um evento posterior que masque a omissão.

### R10 — [P2] Conciliar referências antes de publicar o snapshot

**Local:** [shell-hyprland/src/lib.rs:147](../../crates/shell-hyprland/src/lib.rs#L147), linhas 147–159. **Gate:** G05.

O snapshot junta cinco consultas IPC sequenciais, mas não valida suas relações. Se a janela fechar entre `activewindow` e `clients`, a lista de janelas já vem sem ela, enquanto `focused_window` e `fullscreen` continuam apontando para a entidade desaparecida. Existem corridas equivalentes entre monitores e workspaces. Recarregar todo o estado não torna essas consultas atômicas.

**Reprodução confirmada:** servidor IPC temporário responde com a janela fullscreen `0xabc` em `activewindow` e `[]` em `clients`; o resultado ainda possui `focused_window=Some(0xabc)` e fullscreen ativo, apesar de não possuir janelas.

**Correção sugerida:** validar/conciliar referências contra as coleções obtidas, usando uma janela canônica da lista quando possível, e repetir a leitura de forma limitada quando houver inconsistência. Testar remoção de janela/output durante a coleta.

### R11 — [P2] Manter o listener vivo após o fechamento das últimas surfaces

**Local:** [surface-topology-poc.rs:483](../../crates/shell-platform/src/bin/surface-topology-poc.rs#L483), linhas 483–485. **Gate:** G04.

Quando o compositor envia `Closed` para as últimas surfaces, o PoC põe `running=false`. Se isso ocorrer durante a remoção do último output, mesmo com a conexão Wayland ainda válida, o processo encerra antes que uma reconexão de monitor possa gerar novas surfaces. Esse resultado também depende da ordem entre `Closed` e `GlobalRemove`, pois o caminho de `GlobalRemove` não encerra o loop.

**Validação:** inspeção dos dois caminhos de evento e da condição do loop. Hotplug real não foi executado nesta revisão.

**Correção sugerida:** tratar zero outputs/surfaces como estado transitório válido e continuar acompanhando o registry até desligamento explícito ou perda da conexão.

## Validação executada

Os checks existentes passaram, todos com exit code 0:

```text
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

A suíte existente contém 20 testes. `shell-app` e `surface-topology-poc` não têm testes, e os testes existentes do adapter não exercitam sockets nem interleaving de respostas.

Foi criado um harness temporário em `/tmp/shell-m2-m3-review`, usando os crates do workspace por path e incluindo o arquivo original de tradução. Ele contém 10 casos adicionais, todos falhando nas asserções de comportamento esperado: escala, rotação, foco do output, atomicidade do registro, foco de workspace nomeado, associação de workspace especial, flag da janela focada, maximização, janela desaparecida e evento de título.

```text
cargo test --offline \
  --manifest-path /tmp/shell-m2-m3-review/Cargo.toml \
  --target-dir /home/brasga/projects/shell/target \
  review:: -- --test-threads=1
```

Na primeira execução, dois casos foram bloqueados pelo sandbox ao criar sockets Unix. Após execução autorizada fora do sandbox, ambos chegaram às asserções e reproduziram os bugs, sem acesso ao compositor real. O harness é temporário e não foi acrescentado à suíte do projeto.

## Limites e gates

- Esta revisão não alterou configuração, foco, outputs ou surfaces da sessão real.
- Não houve validação visual, hotplug físico, retomada após restart do compositor ou comparação interativa entre as duas topologias.
- O PoC M2 ignora eventos de escala e não renderiza conteúdo que permita aferir alinhamento/texto/hit testing. O teste numérico de round-trip não comprova fractional scaling no compositor.
- `ShellApplication` oferece `apply_compositor_event`, mas o executável atual não conecta o event stream a um loop de aplicação. A demonstração contínua de eventos está somente em `hyprland-poc --events`; o fluxo completo até o frontend ainda precisa ser demonstrado.
- A reconexão atual faz uma tentativa imediata ao mesmo endpoint e não emite um snapshot de ressíncronização ao reconectar. O relatório M3 registra esse escopo limitado; ele não constitui evidência de recuperação após restart real.
- G03/G04/G05 já estavam abertos nos relatórios. Além de obter a evidência de execução pendente, é necessário corrigir os achados acima. ADR-005 permanece `Proposed`.

## Ordem sugerida para correção

1. R01–R03: corrigir geometria, foco por monitor e captura de input.
2. R04/R11: estabilizar aplicação de snapshots e lifecycle de outputs.
3. R05–R10: corrigir tradução, comandos, invalidação por eventos e consistência do estado.
4. Incorporar os casos reproduzidos à suíte e executar a matriz ao vivo de G03/G04/G05.

## Correções aplicadas

Os achados R01–R11 foram corrigidos no workspace:

- **R01:** o adapter converte resolução física para geometria lógica,
  respeitando escala e transformações de 90/270 graus; há testes para escala
  `1.25` e rotação.
- **R02:** snapshots aplicam `focused_output` atomicamente no
  `OutputRegistry`, incluindo o caso explícito sem foco.
- **R03:** o PoC inicia todas as surfaces com input vazio, configura hitboxes
  após o `configure` e mantém `Overlay`/modo click-through sem input.
- **R04:** `OutputRegistry` reconcilia em uma cópia provisória e só publica
  mutações quando o snapshot inteiro é válido.
- **R05/R06:** IDs negativos de workspaces são preservados; foco resolve
  workspaces nomeados para `name:` e especiais para `special:`.
- **R07:** a janela retornada por `activewindow` é marcada como focada.
- **R08:** os modos do protocolo são diferenciados: `1` é maximizado e `2/3`
  são fullscreen; `Window` agora expõe ambos os estados.
- **R09:** `windowtitle`, `windowtitlev2` e `configreloaded` invalidam o
  snapshot através do event stream.
- **R10:** a coleta de snapshot faz uma segunda tentativa quando referências
  ficam inconsistentes e canonicaliza referências desaparecidas antes de
  publicar o estado.
- **R11:** `Closed` não encerra mais o PoC quando não há surfaces; surfaces são
  recriadas se o output ainda existir e o loop continua aguardando hotplug.

## Validação após as correções

Todos os gates locais passaram:

```text
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

A suíte agora inclui testes de foco atômico, rollback transacional, geometria
com escala/rotação, IDs negativos, estados maximizado/fullscreen, eventos de
título/reload, seleção de workspace nomeado e canonicalização de janela
desaparecida. A validação visual/hotplug e a matriz runtime de G03/G04/G05
continuam dependentes de uma sessão Hyprland real.
