# Linux Shell UI — Project Context

## 1. Objetivo

Construir uma **desktop shell moderna para Linux/Wayland**, inicialmente focada em **Hyprland**, escrita predominantemente em **Rust**.

O projeto deve usar o **Ambxst** como referência funcional e arquitetural, mas não portar QML literalmente. A intenção é reproduzir e evoluir seus conceitos — panel, notch, dock, launcher, dashboard, notifications, OSD, system controls etc. — usando uma arquitetura adequada ao ecossistema Rust.

Frontend inicial:

```text
GPUI + Wayland
```

Integrações e lógica:

```text
Rust
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

---

# 2. Referência: Ambxst

**Fato confirmado:** Ambxst é uma shell Wayland baseada em Quickshell/QtQuick organizada aproximadamente em:

```text
Ambxst
├── shell.qml
├── config/
├── modules/
│   ├── bar/
│   ├── components/
│   ├── corners/
│   ├── desktop/
│   ├── dock/
│   ├── frame/
│   ├── globals/
│   ├── lockscreen/
│   ├── notch/
│   ├── notifications/
│   ├── services/
│   ├── shell/
│   ├── sidebar/
│   ├── theme/
│   ├── tools/
│   └── widgets/
├── backend/
├── assets/
└── scripts/
```



Essa divisão deve servir como **referência de capacidades**, não como estrutura obrigatória do código Rust.

---

# 3. Princípio arquitetural

O projeto deve seguir uma arquitetura próxima de:

```text
                    ┌─────────────────────┐
                    │      UI / GPUI      │
                    │ panel/notch/widgets │
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
       Hyprland IPC       D-Bus/PipeWire       TOML/JSON
```

GPUI é um **presentation adapter**, não parte do domínio.

Não permitir dependências como:

```text
core -> gpui
core -> Hyprland
core -> PipeWire
```

Preferir:

```text
gpui -> core

hyprland-adapter -> core ports
linux-adapters   -> core ports
```

---

# 4. Composition Root

O Ambxst usa `shell.qml` como composição principal da shell e instancia vários componentes por monitor usando `Variants` sobre as telas disponíveis. Wallpaper, desktop, panel, overview, screenshot overlay e OSD seguem esse padrão.

No projeto Rust, o equivalente deve existir no crate executável:

```text
shell-app/
    main.rs
```

Responsabilidades:

```text
- inicializar runtime
- detectar outputs Wayland
- iniciar adapters
- carregar configuração
- criar ShellState
- criar surfaces por monitor
- iniciar GPUI
- conectar eventos do sistema ao core
```

`main.rs` não deve conter regras de negócio.

---

# 5. Modelo de estado

O Ambxst separa conceitualmente três categorias importantes.

## Persistent Configuration

Equivalente ao `Config.qml`.

Ambxst mantém domínios separados como:

```text
bar
theme
ai
compositor
dock
notch
desktop
overview
notifications
tools
lockscreen
system
weather
```

com defaults, validação e persistência reativa.

No projeto Rust:

```rust
struct ShellConfig {
    appearance: AppearanceConfig,
    panel: PanelConfig,
    notch: NotchConfig,
    dock: DockConfig,
    launcher: LauncherConfig,
    notifications: NotificationConfig,
    compositor: CompositorConfig,
}
```

Usar preferencialmente:

```text
serde
toml
```

Configuração persistente não deve ficar armazenada dentro de componentes GPUI.

---

## Runtime State

Equivalente conceitual ao `GlobalStates`.

Exemplos:

```text
focused monitor
active workspace
focused window

launcher open
dashboard open
overview open
settings open

notch state
OSD state

current media
volume
brightness
battery

network
bluetooth

fullscreen state
```

Ambxst também mantém estados como launcher selecionado, dashboard atual, lockscreen, OSD, screenshot e assistant/sidebar separadamente da configuração permanente.

No Rust:

```rust
struct ShellState {
    compositor: CompositorState,
    surfaces: SurfaceState,
    media: MediaState,
    audio: AudioState,
    network: NetworkState,
    bluetooth: BluetoothState,
    power: PowerState,
}
```

A UI deve **renderizar estado**, não descobri-lo diretamente.

---

# 6. Surface / Panel Architecture

Uma das decisões mais interessantes do Ambxst é `UnifiedShellPanel`.

**Fato confirmado:** por monitor, ele cria um `PanelWindow` transparente ocupando toda a tela e coloca dentro dele componentes como:

```text
Bar
Notch
Dock
Sidebar
```

A surface opera na layer `Overlay`.

Quando nada especial está aberto, sua região de input fica restrita aos hitboxes efetivamente interativos:

```text
barHitbox
notchHitbox
dockHitbox
sidebar hitbox
```

Quando um popup/notch precisa detectar clique fora, a região passa temporariamente a cobrir a tela inteira.

### Meta para a implementação Rust

Preservar esse conceito:

```text
Output
└── ShellSurface
    ├── Panel
    ├── Notch
    ├── Dock
    ├── Overlay
    └── Popups
```

Mas validar via PoC se GPUI lida melhor com:

```text
A. uma surface fullscreen por output
```

ou:

```text
B. surfaces LayerShell independentes
   ├── panel
   ├── notch
   ├── dock
   └── overlays
```

Não assumir que a arquitetura de surfaces do Quickshell é automaticamente a melhor para GPUI.

---

# 7. Layer Shell

Para componentes tradicionais da shell:

```text
Panel:
Layer::Top
anchor = TOP | LEFT | RIGHT
exclusive_zone = panel_height

Notch:
Layer::Overlay
anchor = TOP
exclusive_zone = none

Dock:
Layer::Top ou Overlay
dependendo do comportamento

Overlay:
Layer::Overlay
```

Manter uma representação independente de GPUI:

```rust
struct SurfaceSpec {
    layer: ShellLayer,
    anchors: Anchors,
    exclusive_zone: Option<i32>,
    keyboard: KeyboardMode,
    input_region: InputRegion,
}
```

Essa abstração representa **semântica Wayland**, não widgets.

Assim poderá existir:

```text
SurfaceSpec
├── GPUI LayerShell adapter
└── SCTK LayerSurface adapter
```

---

# 8. Focus e Input

Ambxst altera dinamicamente o foco de teclado.

Quando o notch ou sidebar necessita input:

```text
keyboard focus = Exclusive
```

Caso contrário:

```text
keyboard focus = None
```



O projeto deve possuir um coordenador central:

```rust
struct FocusManager {
    active_surface: Option<SurfaceId>,
    active_overlay: Option<OverlayId>,
}
```

Responsável por:

```text
keyboard focus
click outside
modal ownership
popup ordering
input regions
dismiss behavior
```

Não deixar cada widget controlar o Wayland diretamente.

---

# 9. Notch

O notch do Ambxst deve ser usado como referência visual e comportamental.

Ele funciona como um container dinâmico capaz de apresentar diferentes views através de uma navegação semelhante a uma stack:

```text
Default
Launcher
Dashboard
Power Menu
Tools
Notifications
```



Sua largura e altura respondem ao conteúdo e são animadas.

## Geometria

No tema padrão, o formato não é apenas um retângulo arredondado.

O Ambxst constrói uma máscara formada por:

```text
left concave corner
+
center rectangle
+
right concave corner
```

usando `RoundCorner` nas extremidades e uma máscara aplicada ao background.

No Rust/GPUI, implementar isso como uma primitive própria:

```text
NotchShape
```

e não como várias entidades de domínio.

Possíveis implementações:

```text
GPUI Path
custom paint element
GPU path/tessellation
shader
```

A abstração importante é:

```rust
struct NotchGeometry {
    width: f32,
    height: f32,
    radius: f32,
    corner_size: f32,
    position: Edge,
}
```

A UI calcula o path a partir desses dados.

---

# 10. Navegação do notch

Não replicar diretamente `StackView`.

Criar conceito próprio:

```rust
enum NotchRoute {
    Idle,
    Launcher,
    Dashboard,
    Notifications,
    PowerMenu,
    Tools,
}
```

com:

```rust
struct NotchState {
    route: NotchRoute,
    expanded: bool,
}
```

GPUI apenas renderiza esse estado.

---

# 11. Animações

Ambxst anima propriedades como:

```text
width
height
corner radius
blur
```

e utiliza diferentes easings para expansão/retração.

Criar uma pequena camada reutilizável:

```text
animation/
├── easing.rs
├── spring.rs
├── transition.rs
└── timeline.rs
```

Não acoplar a semântica da aplicação às animações.

Exemplo:

```text
Notch state:
Idle -> Launcher
```

não deve significar:

```text
width = 500
height = 420
```

O componente visual decide essas dimensões.

---

# 12. UI primitives

O Ambxst possui uma pasta explícita de componentes reutilizáveis com elementos como:

```text
StyledRect
BarPopup
SearchInput
SegmentedSwitch
Slider
ToggleButton
ContextMenu
Shadow
Tooltip
```

e também shaders próprios para gradientes e efeitos.

Adotar o mesmo princípio em Rust:

```text
ui/
├── primitives/
│   ├── surface.rs
│   ├── button.rs
│   ├── icon.rs
│   ├── text.rs
│   ├── slider.rs
│   ├── popup.rs
│   └── input.rs
│
├── components/
└── features/
```

### Regra

Feature code deve preferir primitives da shell em vez de estilizar tudo individualmente.

---

# 13. Design System

O Ambxst centraliza cores, ícones e styling no módulo `theme`, incluindo `Colors`, `Icons` e `Styling`.

Criar:

```text
shell-theme/
├── colors.rs
├── typography.rs
├── spacing.rs
├── radius.rs
├── shadows.rs
├── motion.rs
└── icons.rs
```

Exemplo:

```rust
struct Theme {
    colors: Colors,
    typography: Typography,
    radius: RadiusScale,
    spacing: SpacingScale,
    motion: MotionTokens,
}
```

Nenhuma feature deve espalhar magic numbers de styling.

---

# 14. Features principais

Usar o Ambxst como catálogo inicial de capabilities:

```text
Panel
├── Workspaces
├── Clock
├── System tray
├── Battery
├── Audio
├── Microphone
├── Power profile
└── Controls

Notch
├── Default view
├── Launcher
├── Dashboard
├── Notifications
├── Power menu
└── Tools

Dock

Notifications

OSD
├── Volume
├── Microphone
└── Brightness

Desktop

Overview

Lockscreen

Settings

Screenshot

Screen recording
```

Ambxst possui módulos explícitos para essas responsabilidades.

Não implementar todos simultaneamente.

---

# 15. Services

Ambxst possui uma camada extensa de services para integrar UI e sistema, incluindo:

```text
Audio
Battery
Bluetooth
Brightness
Clipboard
Compositor
Idle
MPRIS
Network
Notifications
Power profile
Screen recorder
Screenshot
System resources
Wallpaper
Weather
```



A implementação Rust deve ser mais rigorosa.

Cada integração externa deve possuir um port.

Exemplo:

```rust
trait AudioService {
    fn state(&self) -> AudioState;
    async fn set_volume(&self, volume: f32);
    async fn toggle_mute(&self);
}

trait NetworkService {
    async fn scan(&self);
    async fn connect(&self, network: NetworkId);
}

trait CompositorService {
    fn state(&self) -> CompositorState;
    async fn dispatch(&self, command: CompositorCommand);
}
```

---

# 16. Hyprland Adapter

Hyprland não deve aparecer diretamente dentro dos widgets.

Criar:

```text
platform-hyprland/
├── ipc.rs
├── events.rs
├── commands.rs
├── monitors.rs
├── workspaces.rs
└── windows.rs
```

Transformar eventos específicos do Hyprland em eventos internos:

```text
Hyprland workspace event
        ↓
WorkspaceChanged
        ↓
ShellState
        ↓
GPUI rerender
```

Essa tradução é fundamental para permitir outros compositores futuramente.

---

# 17. Compositor abstraction

O próprio Ambxst já possui um `AxctlService` como camada de abstração para operações relacionadas ao compositor.

O projeto Rust deve levar essa ideia mais longe:

```rust
trait Compositor {
    fn monitors(&self) -> Vec<Monitor>;
    fn workspaces(&self) -> Vec<Workspace>;
    fn windows(&self) -> Vec<Window>;

    async fn focus_workspace(&self, id: WorkspaceId);
    async fn focus_window(&self, id: WindowId);
    async fn dispatch(&self, cmd: CompositorCommand);
}
```

Implementação inicial:

```text
HyprlandCompositor
```

Futuramente:

```text
NiriCompositor
SwayCompositor
```

---

# 18. Backend

O Ambxst atual possui um backend separado em Go e concentra ali operações que não deveriam ficar espalhadas pela UI; o repositório também mantém helpers residuais em scripts.

Neste projeto **não criar um daemon separado por padrão**.

Como toda a aplicação já será Rust:

```text
shell process
├── UI
├── application core
├── compositor adapter
└── Linux services
```

Só separar um daemon quando existir justificativa concreta, por exemplo:

```text
privilege boundary
crash isolation
persistent service independent from UI
IPC externo
security
```

Não reproduzir arquitetura multiprocesso apenas porque Ambxst utiliza backend separado.

---

# 19. Linux services

Preferência inicial:

```text
D-Bus -> zbus
NetworkManager -> zbus
BlueZ -> zbus
UPower -> zbus
MPRIS -> zbus

Audio mixer -> PipeWire

Wayland -> GPUI / wayland-client
Hyprland -> Unix IPC
```

Evitar executar ferramentas CLI dentro da camada visual.

Anti-pattern:

```rust
Button::on_click(|| {
    Command::new("wpctl")...
});
```

Preferir:

```text
button
  ↓
AudioCommand::IncreaseVolume
  ↓
AudioService
  ↓
PipeWire adapter
```

---

# 20. Multi-monitor

Multi-monitor deve ser uma preocupação de primeira classe.

Ambxst instancia surfaces e overlays de forma independente por `Quickshell.screens`.

Modelo recomendado:

```rust
struct OutputState {
    id: OutputId,
    geometry: Rect,
    scale: f32,
    panel: PanelState,
    notch: NotchState,
    dock: DockState,
}
```

E:

```rust
struct ShellState {
    outputs: HashMap<OutputId, OutputState>,
    focused_output: Option<OutputId>,
}
```

Não usar um único estado global implícito para posição/escala do monitor.

---

# 21. Performance

Ambxst utiliza loading condicional para diversas views e inclusive diferencia inicialização de serviços críticos e não críticos.

Preservar a estratégia:

```text
startup
├── compositor
├── config
├── outputs
├── panel
└── essential services

lazy
├── settings
├── dashboard pages
├── wallpaper browser
├── screenshot UI
└── heavy tools
```

Não inicializar tudo no startup apenas porque Rust permite.

A shell deve permanecer event-driven.

---

# 22. Workspace Rust proposto

```text
shell/
├── Cargo.toml
│
├── crates/
│   ├── shell-core/
│   │   ├── state/
│   │   ├── events/
│   │   ├── commands/
│   │   └── ports/
│   │
│   ├── shell-platform/
│   │   ├── surfaces.rs
│   │   ├── outputs.rs
│   │   └── geometry.rs
│   │
│   ├── shell-hyprland/
│   │   ├── ipc.rs
│   │   ├── events.rs
│   │   └── commands.rs
│   │
│   ├── shell-linux/
│   │   ├── audio/
│   │   ├── network/
│   │   ├── bluetooth/
│   │   ├── power/
│   │   ├── notifications/
│   │   └── media/
│   │
│   ├── shell-config/
│   │
│   ├── shell-theme/
│   │
│   ├── shell-ui/
│   │   ├── primitives/
│   │   ├── components/
│   │   └── features/
│   │
│   ├── shell-ui-gpui/
│   │   ├── platform/
│   │   ├── surfaces/
│   │   └── renderer/
│   │
│   └── shell-app/
│       └── main.rs
```

Evitar criar dezenas de crates pequenos sem necessidade.

Essas divisões representam **boundaries arquiteturais**; inicialmente alguns módulos podem coexistir no mesmo crate.

---

# 23. Fluxo de dados

Usar fluxo unidirecional sempre que razoável:

```text
Linux / Hyprland event
        ↓
Adapter
        ↓
Domain Event
        ↓
Application
        ↓
ShellState
        ↓
GPUI
```

Comandos seguem sentido oposto:

```text
User interaction
       ↓
UI Intent
       ↓
Application Command
       ↓
Port
       ↓
Linux / Hyprland Adapter
```

Widgets não devem conhecer detalhes de infraestrutura.

---

# 24. Invariantes do projeto

1. **GPUI é substituível.**
2. **Hyprland é um adapter, não o domínio.**
3. **Nenhum widget executa comandos Linux diretamente.**
4. **Config persistente e runtime state são conceitos diferentes.**
5. **Cada monitor possui estado explícito.**
6. **Wayland surface semantics ficam fora dos widgets.**
7. **Design tokens ficam centralizados.**
8. **Features complexas são compostas por primitives reutilizáveis.**
9. **Serviços atualizam o estado; UI observa o estado.**
10. **Não construir um toolkit próprio prematuramente.**

---

# 25. Anti-patterns

Evitar:

```text
GPUI types no core

Hyprland IPC dentro de components

Command::new() espalhado pela UI

um GlobalState gigante contendo tudo

widgets acessando serviços arbitrariamente

config e runtime state misturados

uma abstraction layer genérica para Button/Div/Flex
só para uma futura migração do GPUI

polling onde existe event/subscription

um processo independente para cada utility

uma crate para cada arquivo

copiar a arquitetura QML literalmente
```

---

# 26. Roadmap inicial

## PoC 1 — Layer Shell

Objetivo:

```text
abrir uma surface GPUI no Hyprland
```

Validar:

```text
Layer::Top
Layer::Overlay
anchors
exclusive zone
transparency
input
multi-monitor
fractional scaling
```

---

## PoC 2 — Panel

Criar:

```text
[ workspaces ]                 [ clock | volume | network ]
```

Consumir estado fake inicialmente.

---

## PoC 3 — Hyprland

Adicionar adapter real.

Mostrar:

```text
workspaces
focused workspace
focused window
fullscreen
focused monitor
```

Tudo event-driven.

---

## PoC 4 — Notch

Implementar somente:

```text
Idle
     ↓ click
Launcher
```

Validar:

```text
custom geometry
width animation
height animation
concave corners
input region
focus
click outside
```

Este PoC decide se GPUI atende o nível visual esperado.

---

## PoC 5 — Linux services

Começar por:

```text
Audio
Battery
Network
MPRIS
```

A UI não deve saber qual backend implementa cada serviço.

---

## PoC 6 — Shell MVP

```text
Panel
Notch
Launcher
Notifications
OSD
Audio
Network
Bluetooth
Battery
MPRIS
Hyprland workspaces
```

Só depois considerar:

```text
Dock
Dashboard
Overview
Lockscreen
Desktop icons
Screenshot
Screen recording
AI sidebar
Theme editor
```

---

# 27. Decisão estratégica

A referência arquitetural correta a extrair do Ambxst não é:

```text
“reescrever Ambxst em Rust”
```

É:

```text
Ambxst
    ↓ extrair conceitos

composition root
per-output UI
unified surfaces
reactive state
persistent config
system services
reusable primitives
centralized visibility/focus
feature modules
dynamic notch

    ↓ reinterpretar

Rust core
+ GPUI presentation
+ Wayland semantics
+ Hyprland adapter
+ Linux service adapters
```

O projeto deve preservar a **experiência integrada de desktop shell** do Ambxst enquanto estabelece boundaries mais fortes entre UI, domínio, compositor e sistema operacional.

GPUI deve acelerar a primeira implementação.

Ele não deve definir a arquitetura da shell.