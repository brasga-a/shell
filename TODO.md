# TODO — Luna

Checklist pessoal de execução. O `context/milestones.md` continua sendo o roadmap formal; este arquivo serve para organizar o trabalho do dia a dia.

## Agora

- [ ] Finalizar a arquitetura de module crates
- [ ] Validar dependências e workspace com `crates/modules/*`
- [ ] Garantir que `shell-ui-gpui` não dependa de módulos concretos
- [ ] Garantir que `shell-app` faça o registro/composição dos módulos
- [ ] Validar GPUI Kit com o stack atual de GPUI/Wayland
- [ ] Testar compatibilidade do GPUI Kit com layer-shell
- [ ] Definir contrato final de `NotchModule`
- [ ] Implementar `ModuleRegistry`
- [ ] Implementar `ModuleId` + roteamento `NotchRoute::Module(...)`
- [ ] Validar resize dinâmico do Notch conforme o módulo ativo

## Notch

- [ ] Finalizar geometria com cantos côncavos
- [ ] Implementar animação de width/height
- [ ] Usar GPUI Kit motion onde fizer sentido
- [ ] Implementar `Presence`/transições entre módulos
- [ ] Centralizar focus, Escape e click-outside
- [ ] Validar input regions transparentes
- [ ] Validar comportamento em fullscreen
- [ ] Validar multi-monitor
- [ ] Validar fractional scaling

## GPUI Kit

- [ ] Definir a versão/pin oficial do `gpui-kit`
- [ ] Evitar versões incompatíveis de GPUI no mesmo workspace
- [ ] Mapear componentes que serão usados diretamente
- [ ] Mapear componentes que precisam de customização/fork local
- [ ] Integrar tema Luna com tokens do GPUI Kit
- [ ] Testar Input
- [ ] Testar Button/IconButton
- [ ] Testar Popover/Dialog
- [ ] Testar Select/Checkbox/Switch/Slider
- [ ] Testar Scroll/List/VirtualList
- [ ] Testar motion/spring/transition

## Clock

- [ ] Implementar `luna-module-clock`
- [ ] Mostrar horário atual
- [ ] Mostrar data
- [ ] Suportar 12h/24h
- [ ] Suportar timezone configurável
- [ ] Adicionar timer
- [ ] Adicionar alarme
- [ ] Adicionar world clocks
- [ ] Garantir atualização eficiente sem polling excessivo

## Launcher

- [ ] Estudar implementação do zlaunch
- [ ] Definir `LauncherProvider`
- [ ] Implementar `ProviderRegistry`
- [ ] Implementar `ApplicationsProvider`
- [ ] Ler `.desktop` entries
- [ ] Resolver ícones
- [ ] Criar índice/cache de aplicações
- [ ] Implementar fuzzy search
- [ ] Implementar ranking
- [ ] Implementar keyboard navigation
- [ ] Implementar launch intent sem `Command::new()` dentro da UI
- [ ] Adicionar `WindowsProvider`
- [ ] Adicionar `CalculatorProvider`
- [ ] Adicionar `FilesProvider`
- [ ] Adicionar `CommandsProvider`
- [ ] Adicionar `ClipboardProvider`
- [ ] Adicionar `WebProvider`

## Calendar

- [ ] Implementar `luna-module-calendar`
- [ ] Criar mini month view
- [ ] Mostrar agenda do dia
- [ ] Mostrar próximos eventos
- [ ] Implementar quick create
- [ ] Definir `CalendarCore`
- [ ] Definir `CalendarRepository`
- [ ] Implementar SQLite local-first
- [ ] Definir modelo `Event`, `Calendar`, `Account`, `SyncState`
- [ ] Implementar recorrência corretamente
- [ ] Implementar timezone/all-day corretamente
- [ ] Criar `SyncEngine`
- [ ] Implementar Google Calendar provider
- [ ] Implementar Microsoft/Outlook provider
- [ ] Implementar CalDAV/iCloud provider
- [ ] Criar ação `Open Calendar`
- [ ] Planejar futuro `luna-calendar` completo

## Player

- [ ] Implementar `MediaPort`
- [ ] Implementar adapter MPRIS
- [ ] Implementar `luna-module-player`
- [ ] Mostrar player ativo
- [ ] Mostrar título/artista/album art
- [ ] Play/Pause
- [ ] Previous/Next
- [ ] Seek/progresso quando suportado
- [ ] Seleção entre múltiplos players
- [ ] Estado vazio sem player ativo
- [ ] Avaliar player completo futuro separado da shell

## Resources

- [ ] Definir resource metrics service
- [ ] Coletar CPU
- [ ] Coletar RAM
- [ ] Coletar swap
- [ ] Coletar disk/network activity
- [ ] Coletar GPU quando disponível
- [ ] Coletar temperaturas quando disponível
- [ ] Implementar histórico limitado
- [ ] Implementar sampling configurável
- [ ] Implementar lista de processos
- [ ] Ordenar por CPU/RAM
- [ ] Criar `ProcessPort` antes de permitir kill/signal
- [ ] Planejar futuro Task Manager completo

## Theme

- [ ] Definir modelo final de `Theme`
- [ ] Integrar `shell-theme` com GPUI Kit
- [ ] Mapear tokens semânticos
- [ ] Aplicar tema em Panel/Notch/módulos
- [ ] Implementar presets
- [ ] Implementar preview
- [ ] Implementar edição de cores
- [ ] Implementar radius/spacing/motion configuráveis onde fizer sentido
- [ ] Integrar com `theme.toml`
- [ ] Garantir hot reload sem restart
- [ ] Avaliar futuro Appearance app

## Settings

- [ ] Implementar `luna-module-settings`
- [ ] General settings
- [ ] Modules enable/disable
- [ ] Appearance entry point
- [ ] Keybinds
- [ ] Audio/network/bluetooth quick settings
- [ ] Expor apenas capabilities suportadas pelo compositor
- [ ] Usar config tipada
- [ ] Implementar apply/revert
- [ ] Mostrar erros de validação
- [ ] Planejar futuro `luna-settings` completo

## Linux services

- [ ] Audio / PipeWire
- [ ] Battery / UPower
- [ ] Network / NetworkManager
- [ ] Bluetooth / BlueZ
- [ ] Media / MPRIS
- [ ] Brightness
- [ ] Notifications
- [ ] System tray / StatusNotifierItem
- [ ] Clipboard
- [ ] Resource metrics
- [ ] Garantir uma fonte autoritativa de estado por subsistema
- [ ] Remover CLIs temporárias quando houver API/protocolo adequado

## Configuração

- [ ] Finalizar `ConfigLoader`
- [ ] Finalizar defaults
- [ ] Finalizar validation
- [ ] Finalizar migrations/versioning
- [ ] Watch `$XDG_CONFIG_HOME/luna/`
- [ ] Debounce de mudanças
- [ ] Recarregar config completa inicialmente
- [ ] Manter snapshot anterior em config inválida
- [ ] Atomic swap de config validada
- [ ] Emitir `ConfigChanged`
- [ ] Classificar mudanças em hot reload / recreate surface / restart service / restart shell

## Apps futuros

- [ ] `luna-calendar`
- [ ] `luna-files`
- [ ] `luna-notes`
- [ ] `luna-settings`
- [ ] `luna-task-manager`
- [ ] Definir crates de domínio compartilhados entre shell modules e apps
- [ ] Evitar duplicar storage/sync entre module e app

## Shell infrastructure

- [ ] Panel
- [ ] Workspaces
- [ ] Notifications
- [ ] OSD
- [ ] Tray
- [ ] Focus manager
- [ ] Multi-monitor
- [ ] Fullscreen behavior
- [ ] Output hotplug
- [ ] Fractional scaling
- [ ] Lockscreen via Wayland session-lock

## Qualidade

- [ ] `cargo check --workspace`
- [ ] `cargo test --workspace`
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --workspace`
- [ ] Auditar `unwrap()` / `expect()` em production paths
- [ ] Testar subsystem unavailable states
- [ ] Testar resize/interrupção de animações
- [ ] Testar mudança rápida entre módulos
- [ ] Testar memory growth
- [ ] Medir idle CPU/RAM
- [ ] Medir startup
- [ ] Medir input/module switch latency

## Docs / arquitetura

- [ ] Manter `context/README.md` sincronizado com implementação
- [ ] Atualizar ADRs quando decisões mudarem
- [ ] Atualizar `milestones.md` quando escopo mudar
- [ ] Atualizar docs dos módulos conforme implementação
- [ ] Não mudar boundaries silenciosamente no código

## Backlog

- [ ] Dock
- [ ] Overview
- [ ] Screenshot
- [ ] Screen recording
- [ ] Weather module
- [ ] Clipboard module
- [ ] Power module
- [ ] Network module
- [ ] Bluetooth module
- [ ] AI/assistant module
- [ ] Plugin system real, somente se houver necessidade concreta

## Regra pessoal

Quando estiver em dúvida sobre o que fazer a seguir:

1. escolher **uma** checkbox de `Agora`;
2. terminar até existir um resultado testável;
3. rodar checks relevantes;
4. commitar;
5. só então puxar a próxima tarefa.
