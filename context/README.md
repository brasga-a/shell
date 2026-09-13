# Linux Shell UI — Project Context

## 1. Objetivo

Construir uma **desktop shell moderna para Linux/Wayland**, inicialmente focada em **Hyprland**, escrita predominantemente em **Rust**.

Frontend inicial:

```text
GPUI + Wayland
```

Compositor inicial:

```text
Hyprland
```

A arquitetura deve permitir substituir GPUI futuramente por:

```text
Smithay Client Toolkit
+ wayland-client
+ wgpu
+ renderer/UI própria
```

sem reescrever domínio, serviços ou integrações Linux.

O Ambxst continua sendo uma referência funcional e visual, não um alvo de port literal.

---

# 2. Princípio arquitetural

A shell segue arquitetura hexagonal: domínio e aplicação ficam independentes de GPUI, Hyprland, PipeWire e demais APIs externas.

```text
                  ┌─────────────────────┐
                  │       Modules       │
                  │ individual crates   │
                  └──────────┬──────────┘
                             │
                             ▼
                  ┌─────────────────────┐
                  │  Notch / UI Host    │
                  │   shell-ui-gpui     │
                  └──────────┬──────────┘
                             │
                             ▼
                  ┌─────────────────────┐
                  │ Application / Core  │
                  │ state + use cases   │
                  └──────────┬──────────┘
                             │
          ┌──────────────────┼───────────────────┐
          ▼                  ▼                   ▼
    CompositorPort      SystemServicePort     ConfigPort
          │                  │                   │
          ▼                  ▼                   ▼
     Hyprland IPC       D-Bus/PipeWire          TOML
```

Não permitir:

```text
shell-core -> gpui
shell-core -> Hyprland
shell-core -> PipeWire
```

GPUI é um presentation adapter. Hyprland e serviços Linux são adapters de infraestrutura.

---

# 3. Composition Root

`shell-app` é o composition root.

Responsabilidades:

```text
inicializar runtime
carregar configuração
iniciar adapters
criar ShellState
iniciar GPUI
criar surfaces por output
instanciar módulos
registrar módulos no Notch host
conectar eventos do sistema ao core
```

`main.rs` não contém regras de negócio.

A composição de módulos ocorre aqui para evitar dependência circular entre o host visual e crates concretos de módulos.

---

# 4. Modelo de estado

Configuração persistente e runtime state permanecem separados.

Configuração:

```text
appearance
panel
notch
modules
keybinds
compositor
```

Runtime state:

```text
focused output
active workspace
focused window
notch route
active module
media
volume
network
bluetooth
battery
```

A UI renderiza estado; ela não descobre infraestrutura diretamente.

---

# 5. Surface / Panel Architecture

Por output, a implementação deve validar via PoC se GPUI funciona melhor com:

```text
A. uma surface fullscreen transparente por output
```

ou:

```text
B. surfaces LayerShell independentes
   ├── panel
   ├── notch
   ├── dock
   └── overlays
```

A decisão deve ser baseada em comportamento real de input, foco, fullscreen, multi-monitor e fractional scaling.

Semântica Wayland fica fora dos widgets:

```rust
struct SurfaceSpec {
    layer: ShellLayer,
    anchors: Anchors,
    exclusive_zone: Option<i32>,
    keyboard: KeyboardMode,
    input_region: InputRegion,
}
```

---

# 6. Notch

O **Notch é a casca visual e o host dos módulos**.

Ele é responsável por:

```text
shape
concave geometry
background
routing
mount/unmount do módulo ativo
focus/input boundary
surface/input region
width/height target
resize animation
dismiss behavior
```

Ele não implementa a lógica funcional de Launcher, Calendar, Player, Theme, Resources, Clock ou Settings.

Fluxo conceitual:

```text
User intent
    ↓
NotchRoute::Module(ModuleId)
    ↓
module crate renders content
    ↓
layout/content size
    ↓
Notch computes target geometry
    ↓
animated resize
```

O módulo determina seu conteúdo e layout; o Notch determina a casca, a geometria final e a transição.

O Notch não deve codificar uma enumeração rígida de todas as telas para sempre. Preferir:

```rust
enum ModuleId {
    Launcher,
    Calendar,
    Player,
    Theme,
    Resources,
    Clock,
    Settings,
}

enum NotchRoute {
    Idle,
    Module(ModuleId),
}
```

Os detalhes estão em [context/modules/README.md](./modules/README.md).

---

# 7. Módulos como crates individuais

**Decisão:** cada feature module é um crate Rust individual.

Estrutura inicial:

```text
crates/
├── modules/
│   ├── launcher/
│   ├── calendar/
│   ├── player/
│   ├── theme/
│   ├── resources/
│   ├── clock/
│   └── settings/
│
├── shell-core/
├── shell-platform/
├── shell-hyprland/
├── shell-linux/
├── shell-config/
├── shell-theme/
├── shell-ui-gpui/
└── shell-app/
```

Packages recomendados:

```text
luna-module-launcher
luna-module-calendar
luna-module-player
luna-module-theme
luna-module-resources
luna-module-clock
luna-module-settings
```

Esses crates continuam sendo estaticamente ligados ao binário por padrão. **Crate individual não significa processo separado, plugin dinâmico ou daemon.**

Cada módulo possui ownership explícito de sua feature:

```text
state específico
commands/intents
layout/content
integração com ports necessários
configuração específica
```

Cada módulo não possui ownership de:

```text
Wayland surface
layer-shell
Notch geometry
Hyprland IPC
D-Bus raw client
PipeWire raw client
config file parsing
```

Essas responsabilidades permanecem nas boundaries de plataforma, serviços e host visual.

---

# 8. Dependency model dos módulos

`shell-ui-gpui` contém o host do Notch, primitives, animation, focus/input coordination e integração GPUI específica.

Ele **não depende dos crates concretos dos módulos**.

```text
shell-core / shell-linux / shell-config / shell-theme
                    ↑
                    │
             module crates
                    ↑
                    │ instantiated by
                 shell-app
                    │
                    ▼
             shell-ui-gpui
              Notch host
```

`shell-app` registra os módulos no host.

Uma interface conceitual pode ser:

```rust
trait NotchModule {
    fn id(&self) -> ModuleId;
    fn title(&self) -> &'static str;
    fn render(&mut self, cx: &mut ModuleContext) -> ModuleView;
}
```

A API concreta pode ser adaptada às limitações e idioms do GPUI. O importante é preservar a boundary.

---

# 9. Módulos iniciais

Os módulos iniciais são:

```text
Launcher
Calendar
Player
Theme
Task Manager / Resources
Clock
Settings
```

Documentação:

- [Launcher](./modules/launcher.md)
- [Calendar](./modules/calendar.md)
- [Player](./modules/player.md)
- [Theme](./modules/theme.md)
- [Task Manager / Resources](./modules/resources.md)
- [Clock](./modules/clock.md)
- [Settings](./modules/settings.md)

Observação importante: o crate de módulo `theme` é a **feature interativa de edição/seleção de tema**. `shell-theme` continua sendo a infraestrutura compartilhada de tokens, modelos e design system.

---

# 10. UI primitives

Primitives compartilhadas permanecem em `shell-ui-gpui` ou em uma boundary compartilhada equivalente, e não são duplicadas em cada módulo.

Exemplos:

```text
surface
button
icon
text
slider
popup
input
tooltip
```

Módulos usam primitives compartilhadas, mas mantêm layout e comportamento específicos dentro de seus próprios crates.

Evitar criar uma abstraction layer genérica para todos os elementos GPUI apenas com o objetivo de uma possível migração futura.

---

# 11. Design System

`shell-theme` continua centralizando:

```text
colors
typography
spacing
radius
shadows
motion
icons
```

Nenhum módulo deve espalhar magic numbers de styling sem motivo.

O módulo `luna-module-theme` consome e edita esses modelos; ele não substitui `shell-theme`.

---

# 12. Services

Integrações externas ficam atrás de ports.

Preferências iniciais:

```text
D-Bus -> zbus
NetworkManager -> zbus
BlueZ -> zbus
UPower -> zbus
MPRIS -> zbus
Audio -> PipeWire
Hyprland -> Unix IPC
Wayland -> GPUI / wayland-client
```

Exemplo correto:

```text
Player module
    ↓
MediaPort
    ↓
shell-linux
    ↓
MPRIS / D-Bus
```

Evitar:

```text
Player widget -> raw D-Bus
Resources widget -> Command::new("ps")
Launcher widget -> hyprctl
```

---

# 13. Focus e input

O host visual possui um coordenador central para:

```text
keyboard focus
click outside
modal ownership
input regions
dismiss behavior
```

Módulos declaram necessidades de interação, mas não alteram layer-shell ou Wayland diretamente.

---

# 14. Multi-monitor

Multi-monitor é uma preocupação de primeira classe.

```rust
struct OutputState {
    id: OutputId,
    geometry: Rect,
    scale: f32,
    panel: PanelState,
    notch: NotchState,
}
```

O módulo ativo deve ser associado ao contexto de output adequado quando necessário.

Não usar estado global implícito para escala, posição ou monitor ativo.

---

# 15. Performance e lifecycle

Os crates são compilados no binário, mas seus estados e recursos podem ser inicializados de forma lazy.

```text
startup
├── compositor
├── config
├── outputs
├── panel/notch host
└── essential services

lazy
├── launcher index
├── calendar data
├── player expanded UI
├── theme editor
├── resources sampling
└── settings UI
```

Um módulo fechado não deve manter trabalho pesado desnecessário apenas porque seu código está linkado ao executável.

---

# 16. Workspace Rust

```text
luna/
├── Cargo.toml
└── crates/
    ├── shell-core/
    ├── shell-platform/
    ├── shell-hyprland/
    ├── shell-linux/
    ├── shell-config/
    ├── shell-theme/
    ├── shell-ui-gpui/
    ├── modules/
    │   ├── launcher/
    │   ├── calendar/
    │   ├── player/
    │   ├── theme/
    │   ├── resources/
    │   ├── clock/
    │   └── settings/
    └── shell-app/
```

Crates de infraestrutura representam boundaries técnicas. Crates de módulos representam boundaries de produto/feature.

Não criar um crate por arquivo ou por primitive visual.

---

# 17. Fluxo de dados

Eventos:

```text
Linux / Hyprland event
        ↓
Adapter
        ↓
Domain Event
        ↓
Application State
        ↓
Module / Notch UI
```

Comandos:

```text
User interaction
       ↓
Module intent
       ↓
Application command
       ↓
Port
       ↓
Linux / Hyprland adapter
```

---

# 18. Invariantes

1. GPUI é substituível.
2. Hyprland é adapter, não domínio.
3. Nenhum módulo executa comandos Linux/Hyprland diretamente a partir de widgets.
4. Config persistente e runtime state são conceitos diferentes.
5. Cada output possui estado explícito.
6. Wayland surface semantics ficam fora dos módulos.
7. Design tokens ficam centralizados.
8. **Cada feature module inicial é um crate individual.**
9. **O Notch é o host/casca; módulos são conteúdo independente.**
10. `shell-ui-gpui` não depende de crates concretos de módulos.
11. `shell-app` registra os módulos no host.
12. Serviços atualizam estado; módulos observam/acionam ports.
13. Crates individuais não implicam processos individuais.
14. Não construir um toolkit próprio prematuramente.

---

# 19. Anti-patterns

Evitar:

```text
GPUI types no core
Hyprland IPC dentro de módulos
Command::new() espalhado pela UI
raw D-Bus dentro de widgets
config e runtime state misturados
módulos controlando Wayland surfaces
Notch importando cada módulo concreto diretamente
um processo independente para cada módulo
uma crate para cada arquivo ou primitive
copiar a arquitetura QML literalmente
```

---

# 20. Roadmap inicial

## PoC 1 — Layer Shell

Validar GPUI + Wayland:

```text
Top/Overlay
anchors
exclusive zone
transparency
input
multi-monitor
fractional scaling
```

## PoC 2 — Notch host

Implementar a casca sem feature complexa:

```text
Idle
Module placeholder
resize
animation
focus
click outside
```

## PoC 3 — Launcher crate

Usar `luna-module-launcher` como primeira prova real da arquitetura:

```text
shell-app registers launcher
        ↓
Notch mounts launcher
        ↓
launcher renders content
        ↓
Notch resizes around it
```

## PoC 4 — Hyprland adapter

Adicionar estado real de compositor sem expor Hyprland aos módulos.

## PoC 5 — Linux services

Começar por:

```text
MPRIS
Audio
Battery
Network
System resources
```

## PoC 6 — Initial module set

Integrar progressivamente:

```text
Launcher
Clock
Player
Calendar
Resources
Settings
Theme
```

---

# 21. Decisão estratégica

A arquitetura não é "reescrever Ambxst em Rust".

É extrair os conceitos úteis e reinterpretá-los com boundaries explícitas:

```text
Rust core
+ GPUI host
+ dynamic Notch
+ individual module crates
+ Wayland semantics
+ Hyprland adapter
+ Linux service adapters
```

O Notch fornece uma experiência visual integrada. Os módulos permanecem independentes como unidades de feature e compilação.

GPUI acelera a primeira implementação, mas não define a arquitetura da shell.