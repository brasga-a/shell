use std::{collections::HashMap, error::Error, os::unix::io::AsFd};

use shell_core::OutputId;
use shell_platform::{Anchors, InputRegion, KeyboardMode, ShellLayer, SurfaceSpec};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, WEnum, delegate_noop,
    globals::{Global, GlobalListContents, registry_queue_init},
    protocol::{
        wl_buffer, wl_compositor, wl_keyboard, wl_output, wl_pointer, wl_region, wl_registry,
        wl_seat, wl_shm, wl_shm_pool, wl_surface,
    },
};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BufferStatus {
    Available,
    Attached,
}

struct BufferSlot {
    buffer: wl_buffer::WlBuffer,
    width: u32,
    height: u32,
    status: BufferStatus,
}

#[derive(Clone, Copy, Debug)]
struct OutputData {
    global_name: u32,
}

struct OutputState {
    proxy: wl_output::WlOutput,
    name: Option<String>,
    advertised_version: u32,
}

#[derive(Clone, Copy, Debug)]
struct SeatData {
    global_name: u32,
}

#[derive(Clone, Copy, Debug)]
struct DeviceData {
    seat_global: u32,
}

struct SeatState {
    proxy: wl_seat::WlSeat,
    pointer: Option<wl_pointer::WlPointer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
}

struct State {
    running: bool,
    surface: wl_surface::WlSurface,
    shm: wl_shm::WlShm,
    layer_surface: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    buffers: HashMap<u32, BufferSlot>,
    next_buffer_id: u32,
    outputs: HashMap<u32, OutputState>,
    seats: HashMap<u32, SeatState>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("layer-shell-poc failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let options = Options::parse(std::env::args().skip(1))?;
    let connection = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init::<State>(&connection)?;
    let queue_handle = event_queue.handle();

    let compositor = globals.bind::<wl_compositor::WlCompositor, _, _>(&queue_handle, 4..=6, ())?;
    let shm = globals.bind::<wl_shm::WlShm, _, _>(&queue_handle, 1..=1, ())?;
    let layer_shell =
        globals.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(&queue_handle, 1..=4, ())?;
    let surface = compositor.create_surface(&queue_handle, ());

    let mut state = State {
        running: true,
        surface,
        shm,
        layer_surface: None,
        buffers: HashMap::new(),
        next_buffer_id: 1,
        outputs: HashMap::new(),
        seats: HashMap::new(),
    };

    bind_initial_outputs_and_seats(&globals, &queue_handle, &mut state);
    event_queue.blocking_dispatch(&mut state)?;

    let output = select_output(&state, options.output_name.as_deref())?;
    let layer_surface = layer_shell.get_layer_surface(
        &state.surface,
        output.as_ref(),
        protocol_layer(options.surface.layer),
        "linux-shell-milestone-1".to_string(),
        &queue_handle,
        (),
    );

    layer_surface.set_anchor(protocol_anchors(options.surface.anchors));
    layer_surface.set_size(0, 32);
    if let Some(exclusive_zone) = options.surface.exclusive_zone {
        layer_surface.set_exclusive_zone(exclusive_zone.max(0));
    }
    layer_surface.set_keyboard_interactivity(protocol_keyboard(options.surface.keyboard));
    apply_input_region(
        &compositor,
        &state.surface,
        &options.surface.input_region,
        &queue_handle,
    );

    state.layer_surface = Some(layer_surface);
    state.surface.commit();

    let selected_output = options
        .output_name
        .as_deref()
        .map(|name| format!("name={name}"))
        .unwrap_or_else(|| "default".to_string());
    eprintln!(
        "layer-shell-poc: layer={:?} keyboard={:?} output={} click_through={}",
        options.surface.layer,
        options.surface.keyboard,
        selected_output,
        matches!(
            options.surface.input_region,
            InputRegion::Rectangles(ref rectangles) if rectangles.is_empty()
        )
    );

    while state.running {
        event_queue.blocking_dispatch(&mut state)?;
    }

    if let Some(layer_surface) = state.layer_surface.take() {
        layer_surface.destroy();
    }
    state.surface.destroy();
    for output in state.outputs.into_values() {
        if output.advertised_version >= 3 {
            output.proxy.release();
        }
    }
    for seat in state.seats.into_values() {
        if let Some(pointer) = seat.pointer {
            pointer.release();
        }
        if let Some(keyboard) = seat.keyboard {
            keyboard.release();
        }
        if seat.proxy.version() >= 5 {
            seat.proxy.release();
        }
    }
    for slot in state.buffers.into_values() {
        if slot.status == BufferStatus::Available {
            slot.buffer.destroy();
        }
    }

    Ok(())
}

fn bind_initial_outputs_and_seats(
    globals: &wayland_client::globals::GlobalList,
    queue_handle: &QueueHandle<State>,
    state: &mut State,
) {
    for global in globals.contents().clone_list() {
        match global.interface.as_str() {
            "wl_output" => bind_output(globals.registry(), queue_handle, state, &global),
            "wl_seat" => bind_seat(globals.registry(), queue_handle, state, &global),
            _ => {}
        }
    }
}

fn bind_output(
    registry: &wl_registry::WlRegistry,
    queue_handle: &QueueHandle<State>,
    state: &mut State,
    global: &Global,
) {
    if state.outputs.contains_key(&global.name) {
        return;
    }
    let advertised_version = global.version.min(4);
    let proxy = registry.bind::<wl_output::WlOutput, _, _>(
        global.name,
        advertised_version,
        queue_handle,
        OutputData {
            global_name: global.name,
        },
    );
    state.outputs.insert(
        global.name,
        OutputState {
            proxy,
            name: None,
            advertised_version,
        },
    );
}

fn bind_seat(
    registry: &wl_registry::WlRegistry,
    queue_handle: &QueueHandle<State>,
    state: &mut State,
    global: &Global,
) {
    if state.seats.contains_key(&global.name) {
        return;
    }
    let proxy = registry.bind::<wl_seat::WlSeat, _, _>(
        global.name,
        global.version.min(5),
        queue_handle,
        SeatData {
            global_name: global.name,
        },
    );
    state.seats.insert(
        global.name,
        SeatState {
            proxy,
            pointer: None,
            keyboard: None,
        },
    );
}

fn select_output(
    state: &State,
    requested_name: Option<&str>,
) -> Result<Option<wl_output::WlOutput>, Box<dyn Error>> {
    let Some(requested_name) = requested_name else {
        return Ok(None);
    };

    state
        .outputs
        .values()
        .find(|output| output.name.as_deref() == Some(requested_name))
        .map(|output| output.proxy.clone())
        .ok_or_else(|| {
            let available = state
                .outputs
                .values()
                .filter_map(|output| output.name.as_deref())
                .collect::<Vec<_>>()
                .join(", ");
            format!("output '{requested_name}' was not found; available outputs: {available}")
                .into()
        })
        .map(Some)
}

fn protocol_layer(layer: ShellLayer) -> zwlr_layer_shell_v1::Layer {
    match layer {
        ShellLayer::Background => zwlr_layer_shell_v1::Layer::Background,
        ShellLayer::Bottom => zwlr_layer_shell_v1::Layer::Bottom,
        ShellLayer::Top => zwlr_layer_shell_v1::Layer::Top,
        ShellLayer::Overlay => zwlr_layer_shell_v1::Layer::Overlay,
    }
}

fn protocol_anchors(anchors: Anchors) -> zwlr_layer_surface_v1::Anchor {
    let mut result = zwlr_layer_surface_v1::Anchor::empty();
    if anchors.top {
        result |= zwlr_layer_surface_v1::Anchor::Top;
    }
    if anchors.bottom {
        result |= zwlr_layer_surface_v1::Anchor::Bottom;
    }
    if anchors.left {
        result |= zwlr_layer_surface_v1::Anchor::Left;
    }
    if anchors.right {
        result |= zwlr_layer_surface_v1::Anchor::Right;
    }
    result
}

fn protocol_keyboard(keyboard: KeyboardMode) -> zwlr_layer_surface_v1::KeyboardInteractivity {
    match keyboard {
        KeyboardMode::None => zwlr_layer_surface_v1::KeyboardInteractivity::None,
        KeyboardMode::Exclusive => zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive,
        KeyboardMode::OnDemand => zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand,
    }
}

fn apply_input_region(
    compositor: &wl_compositor::WlCompositor,
    surface: &wl_surface::WlSurface,
    input_region: &InputRegion,
    queue_handle: &QueueHandle<State>,
) {
    let InputRegion::Rectangles(rectangles) = input_region else {
        return;
    };

    let region = compositor.create_region(queue_handle, ());
    for rectangle in rectangles {
        region.add(
            rectangle.origin.x.floor() as i32,
            rectangle.origin.y.floor() as i32,
            rectangle.size.width.ceil() as i32,
            rectangle.size.height.ceil() as i32,
        );
    }
    surface.set_input_region(Some(&region));
    region.destroy();
}

fn transparent_buffer(
    shm: &wl_shm::WlShm,
    queue_handle: &QueueHandle<State>,
    width: u32,
    height: u32,
    buffer_id: u32,
) -> Result<wl_buffer::WlBuffer, Box<dyn Error>> {
    let file = tempfile::tempfile()?;
    let stride = width
        .checked_mul(4)
        .ok_or("layer-shell-poc stride overflow")?;
    let byte_len = u64::from(stride)
        .checked_mul(u64::from(height))
        .ok_or("layer-shell-poc buffer size overflow")?;
    let pool_size = i32::try_from(byte_len).map_err(|_| "layer-shell-poc buffer is too large")?;
    let width = i32::try_from(width).map_err(|_| "layer-shell-poc width is too large")?;
    let height = i32::try_from(height).map_err(|_| "layer-shell-poc height is too large")?;
    let stride = i32::try_from(stride).map_err(|_| "layer-shell-poc stride is too large")?;
    file.set_len(byte_len)?;
    let pool = shm.create_pool(file.as_fd(), pool_size, queue_handle, ());
    let buffer = pool.create_buffer(
        0,
        width,
        height,
        stride,
        wl_shm::Format::Argb8888,
        queue_handle,
        buffer_id,
    );
    pool.destroy();
    Ok(buffer)
}

struct Options {
    surface: SurfaceSpec,
    output_name: Option<String>,
}

impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let mut options = Self {
            surface: SurfaceSpec {
                output: OutputId::new(0),
                layer: ShellLayer::Top,
                anchors: Anchors::TOP_LEFT_RIGHT,
                exclusive_zone: Some(32),
                keyboard: KeyboardMode::None,
                input_region: InputRegion::CompositorManaged,
            },
            output_name: None,
        };
        let mut args = args.into_iter();

        while let Some(argument) = args.next() {
            match argument.as_str() {
                "top" => {
                    options.surface.layer = ShellLayer::Top;
                    options.surface.exclusive_zone = Some(32);
                }
                "overlay" => {
                    options.surface.layer = ShellLayer::Overlay;
                    options.surface.exclusive_zone = None;
                }
                "--keyboard=none" => options.surface.keyboard = KeyboardMode::None,
                "--keyboard=exclusive" => options.surface.keyboard = KeyboardMode::Exclusive,
                "--keyboard=on-demand" => options.surface.keyboard = KeyboardMode::OnDemand,
                "--click-through" => {
                    options.surface.input_region = InputRegion::Rectangles(Vec::new());
                }
                "--no-exclusive-zone" => options.surface.exclusive_zone = None,
                "--exclusive-zone" => {
                    let value: i32 = args
                        .next()
                        .ok_or("--exclusive-zone requires a non-negative integer")?
                        .parse()?;
                    if value < 0 {
                        return Err("--exclusive-zone must be non-negative".into());
                    }
                    options.surface.exclusive_zone = Some(value);
                }
                "--output" => {
                    let name = args.next().ok_or("--output requires an output name")?;
                    if name.is_empty() {
                        return Err("--output requires a non-empty output name".into());
                    }
                    options.output_name = Some(name);
                }
                "--help" | "-h" => {
                    println!(
                        "usage: layer-shell-poc [top|overlay] [--keyboard=none|exclusive|on-demand] [--click-through] [--no-exclusive-zone|--exclusive-zone PIXELS] [--output NAME]"
                    );
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument: {unknown}").into()),
            }
        }

        Ok(options)
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &GlobalListContents,
        _connection: &Connection,
        queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                let global = Global {
                    name,
                    interface,
                    version,
                };
                match global.interface.as_str() {
                    "wl_output" => bind_output(registry, queue_handle, state, &global),
                    "wl_seat" => bind_seat(registry, queue_handle, state, &global),
                    _ => {}
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                state.outputs.remove(&name);
                state.seats.remove(&name);
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, OutputData> for State {
    fn event(
        state: &mut Self,
        _output: &wl_output::WlOutput,
        event: wl_output::Event,
        data: &OutputData,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        let Some(output) = state.outputs.get_mut(&data.global_name) else {
            return;
        };
        match event {
            wl_output::Event::Name { name } => {
                output.name = Some(name);
                eprintln!("layer-shell-poc: output name={:?}", output.name);
            }
            wl_output::Event::Description { .. } => {}
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, SeatData> for State {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        data: &SeatData,
        _connection: &Connection,
        queue_handle: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capabilities),
        } = event
        {
            let Some(seat_state) = state.seats.get_mut(&data.global_name) else {
                return;
            };
            if capabilities.contains(wl_seat::Capability::Pointer) && seat_state.pointer.is_none() {
                seat_state.pointer = Some(seat.get_pointer(
                    queue_handle,
                    DeviceData {
                        seat_global: data.global_name,
                    },
                ));
                eprintln!("layer-shell-poc: pointer capability enabled");
            }
            if capabilities.contains(wl_seat::Capability::Keyboard) && seat_state.keyboard.is_none()
            {
                seat_state.keyboard = Some(seat.get_keyboard(
                    queue_handle,
                    DeviceData {
                        seat_global: data.global_name,
                    },
                ));
                eprintln!("layer-shell-poc: keyboard capability enabled");
            }
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, DeviceData> for State {
    fn event(
        state: &mut Self,
        _pointer: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        data: &DeviceData,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface_x,
                surface_y,
                ..
            } => eprintln!(
                "layer-shell-poc: pointer enter seat={} x={surface_x} y={surface_y}",
                data.seat_global
            ),
            wl_pointer::Event::Button {
                button,
                state: button_state,
                ..
            } => eprintln!(
                "layer-shell-poc: pointer button seat={} button={button} state={button_state:?}",
                data.seat_global
            ),
            wl_pointer::Event::Leave { .. } => {
                eprintln!("layer-shell-poc: pointer leave seat={}", data.seat_global)
            }
            _ => {}
        }
        let _ = state;
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, DeviceData> for State {
    fn event(
        state: &mut Self,
        _keyboard: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        data: &DeviceData,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key {
            key,
            state: key_state,
            ..
        } = event
        {
            eprintln!(
                "layer-shell-poc: keyboard key seat={} key={key} state={key_state:?}",
                data.seat_global
            );
            if key == 1 && matches!(key_state, WEnum::Value(wl_keyboard::KeyState::Pressed)) {
                state.running = false;
            }
        }
    }
}

impl Dispatch<wl_buffer::WlBuffer, u32> for State {
    fn event(
        state: &mut Self,
        buffer: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        data: &u32,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = event {
            if let Some(slot) = state.buffers.get_mut(data) {
                slot.status = BufferStatus::Available;
                eprintln!("layer-shell-poc: buffer released id={data}");
            } else {
                buffer.destroy();
            }
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        layer_surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _data: &(),
        _connection: &Connection,
        queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                layer_surface.ack_configure(serial);
                let width = width.max(1);
                let height = height.max(1);
                let buffer_id = match acquire_buffer(state, queue_handle, width, height) {
                    Ok(buffer_id) => buffer_id,
                    Err(error) => {
                        eprintln!("layer-shell-poc: could not allocate buffer: {error}");
                        state.running = false;
                        return;
                    }
                };
                let buffer = &state.buffers[&buffer_id].buffer;
                state.surface.attach(Some(buffer), 0, 0);
                state.surface.damage_buffer(
                    0,
                    0,
                    width.min(i32::MAX as u32) as i32,
                    height.min(i32::MAX as u32) as i32,
                );
                state.surface.commit();
                eprintln!(
                    "layer-shell-poc: configured width={width} height={height} buffer_id={buffer_id}"
                );
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.running = false;
            }
            _ => {}
        }
    }
}

fn acquire_buffer(
    state: &mut State,
    queue_handle: &QueueHandle<State>,
    width: u32,
    height: u32,
) -> Result<u32, Box<dyn Error>> {
    if let Some((id, slot)) = state.buffers.iter_mut().find(|(_, slot)| {
        slot.status == BufferStatus::Available && slot.width == width && slot.height == height
    }) {
        slot.status = BufferStatus::Attached;
        return Ok(*id);
    }

    let stale_ids = state
        .buffers
        .iter()
        .filter(|(_, slot)| {
            slot.status == BufferStatus::Available && (slot.width != width || slot.height != height)
        })
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    for id in stale_ids {
        if let Some(slot) = state.buffers.remove(&id) {
            slot.buffer.destroy();
        }
    }

    let buffer_id = state.next_buffer_id;
    state.next_buffer_id = state
        .next_buffer_id
        .checked_add(1)
        .ok_or("layer-shell-poc buffer id overflow")?;
    let buffer = transparent_buffer(&state.shm, queue_handle, width, height, buffer_id)?;
    state.buffers.insert(
        buffer_id,
        BufferSlot {
            buffer,
            width,
            height,
            status: BufferStatus::Attached,
        },
    );
    Ok(buffer_id)
}

delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_region::WlRegion);
delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_surface::WlSurface);
delegate_noop!(State: ignore zwlr_layer_shell_v1::ZwlrLayerShellV1);

#[cfg(test)]
mod tests {
    use super::{BufferStatus, HashMap};

    #[derive(Default)]
    struct BufferLifecycle {
        states: HashMap<u32, BufferStatus>,
        next_id: u32,
    }

    impl BufferLifecycle {
        fn create(&mut self) -> u32 {
            self.next_id += 1;
            self.states.insert(self.next_id, BufferStatus::Available);
            self.next_id
        }

        fn attach(&mut self, id: u32) {
            self.states.insert(id, BufferStatus::Attached);
        }

        fn release(&mut self, id: u32) {
            self.states.remove(&id);
        }
    }

    #[test]
    fn buffer_lifecycle_is_created_attached_then_released() {
        let mut lifecycle = BufferLifecycle::default();
        let id = lifecycle.create();
        assert_eq!(lifecycle.states.get(&id), Some(&BufferStatus::Available));

        lifecycle.attach(id);
        assert_eq!(lifecycle.states.get(&id), Some(&BufferStatus::Attached));

        lifecycle.release(id);
        assert!(!lifecycle.states.contains_key(&id));
    }
}
