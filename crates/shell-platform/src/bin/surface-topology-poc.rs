//! Live Milestone 2 experiment for unified and independent surface topology.
//!
//! This executable intentionally renders transparent SHM buffers. Its purpose
//! is to measure compositor behavior, output ownership and lifecycle before
//! routing product widgets through the chosen frontend.

use std::{collections::HashMap, error::Error, os::unix::io::AsFd};

use shell_platform::{SurfaceRole, SurfaceTopology};
use wayland_client::{
    Connection, Dispatch, QueueHandle, delegate_noop,
    globals::{Global, GlobalListContents, registry_queue_init},
    protocol::{
        wl_buffer, wl_compositor, wl_output, wl_region, wl_registry, wl_shm, wl_shm_pool,
        wl_surface,
    },
};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct SurfaceKey {
    output_global: u32,
    role: SurfaceRole,
}

#[derive(Clone, Copy, Debug)]
struct OutputData {
    global_name: u32,
}

struct OutputRuntime {
    proxy: wl_output::WlOutput,
    name: Option<String>,
    advertised_version: u32,
}

struct SurfaceRuntime {
    surface: wl_surface::WlSurface,
    layer_surface: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    buffer_id: Option<u32>,
}

struct State {
    running: bool,
    compositor: wl_compositor::WlCompositor,
    shm: wl_shm::WlShm,
    layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1,
    topology: SurfaceTopology,
    click_through: bool,
    exclusive_zone: bool,
    outputs: HashMap<u32, OutputRuntime>,
    surfaces: HashMap<SurfaceKey, SurfaceRuntime>,
    buffers: HashMap<u32, wl_buffer::WlBuffer>,
    buffer_owners: HashMap<u32, SurfaceKey>,
    next_buffer_id: u32,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("surface-topology-poc failed: {error}");
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

    let mut state = State {
        running: true,
        compositor,
        shm,
        layer_shell,
        topology: options.topology,
        click_through: options.click_through,
        exclusive_zone: options.exclusive_zone,
        outputs: HashMap::new(),
        surfaces: HashMap::new(),
        buffers: HashMap::new(),
        buffer_owners: HashMap::new(),
        next_buffer_id: 1,
    };

    for global in globals.contents().clone_list() {
        if global.interface == "wl_output" {
            bind_output(globals.registry(), &queue_handle, &mut state, &global);
        }
    }

    eprintln!(
        "surface-topology-poc: topology={:?} click_through={} exclusive_zone={}",
        state.topology, state.click_through, state.exclusive_zone
    );
    while state.running {
        event_queue.blocking_dispatch(&mut state)?;
    }

    shutdown(&mut state);
    Ok(())
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
        OutputRuntime {
            proxy,
            name: None,
            advertised_version,
        },
    );
}

fn ensure_output_surfaces(
    state: &mut State,
    output_global: u32,
    queue_handle: &QueueHandle<State>,
) {
    let Some(output) = state.outputs.get(&output_global) else {
        return;
    };
    let output_proxy = output.proxy.clone();
    let output_name = output
        .name
        .clone()
        .unwrap_or_else(|| format!("global-{output_global}"));

    for role in state.topology.roles() {
        let key = SurfaceKey {
            output_global,
            role: *role,
        };
        if state.surfaces.contains_key(&key) {
            continue;
        }

        let surface = state.compositor.create_surface(queue_handle, ());
        let layer_surface = state.layer_shell.get_layer_surface(
            &surface,
            Some(&output_proxy),
            protocol_layer(key.role),
            format!("linux-shell-milestone-2-{output_name}-{:?}", key.role),
            queue_handle,
            key,
        );
        configure_layer_surface(state, &surface, &layer_surface, key, queue_handle);
        surface.commit();
        state.surfaces.insert(
            key,
            SurfaceRuntime {
                surface,
                layer_surface,
                buffer_id: None,
            },
        );
        eprintln!(
            "surface-topology-poc: created output={output_name} global={output_global} role={:?}",
            key.role
        );
    }
}

fn configure_layer_surface(
    state: &State,
    surface: &wl_surface::WlSurface,
    layer_surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    key: SurfaceKey,
    queue_handle: &QueueHandle<State>,
) {
    match key.role {
        SurfaceRole::Unified => {
            layer_surface.set_anchor(
                zwlr_layer_surface_v1::Anchor::Top
                    | zwlr_layer_surface_v1::Anchor::Bottom
                    | zwlr_layer_surface_v1::Anchor::Left
                    | zwlr_layer_surface_v1::Anchor::Right,
            );
            layer_surface.set_size(0, 0);
        }
        SurfaceRole::Panel => {
            layer_surface.set_anchor(
                zwlr_layer_surface_v1::Anchor::Top
                    | zwlr_layer_surface_v1::Anchor::Left
                    | zwlr_layer_surface_v1::Anchor::Right,
            );
            layer_surface.set_size(0, 32);
            if state.exclusive_zone {
                layer_surface.set_exclusive_zone(32);
            }
        }
        SurfaceRole::Notch => {
            layer_surface.set_anchor(zwlr_layer_surface_v1::Anchor::Top);
            layer_surface.set_size(420, 96);
        }
        SurfaceRole::Overlay => {
            layer_surface.set_anchor(
                zwlr_layer_surface_v1::Anchor::Top
                    | zwlr_layer_surface_v1::Anchor::Bottom
                    | zwlr_layer_surface_v1::Anchor::Left
                    | zwlr_layer_surface_v1::Anchor::Right,
            );
            layer_surface.set_size(0, 0);
        }
    }

    layer_surface.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);
    // Keep every surface non-interactive until the compositor has sent its
    // final size. Transparent pixels do not imply click-through in Wayland.
    set_empty_input_region(state, surface, queue_handle);
}

fn set_empty_input_region(
    state: &State,
    surface: &wl_surface::WlSurface,
    queue_handle: &QueueHandle<State>,
) {
    let region = state.compositor.create_region(queue_handle, ());
    surface.set_input_region(Some(&region));
    region.destroy();
}

fn configure_input_region(
    state: &State,
    surface: &wl_surface::WlSurface,
    key: SurfaceKey,
    width: u32,
    height: u32,
    queue_handle: &QueueHandle<State>,
) {
    if state.click_through || matches!(key.role, SurfaceRole::Overlay) {
        set_empty_input_region(state, surface, queue_handle);
        return;
    }

    match key.role {
        SurfaceRole::Panel | SurfaceRole::Notch => {
            // These surfaces are already bounded to their interactive shell
            // component, so the default region is the whole surface.
            surface.set_input_region(None);
        }
        SurfaceRole::Unified => {
            let width = width.min(i32::MAX as u32) as i32;
            let height = height.min(i32::MAX as u32) as i32;
            let region = state.compositor.create_region(queue_handle, ());
            let panel_height = height.min(32);
            if panel_height > 0 {
                region.add(0, 0, width, panel_height);
            }
            let notch_width = width.min(420);
            let notch_height = height.min(96);
            if notch_width > 0 && notch_height > 0 {
                region.add((width - notch_width) / 2, 0, notch_width, notch_height);
            }
            surface.set_input_region(Some(&region));
            region.destroy();
        }
        SurfaceRole::Overlay => unreachable!("overlay input region handled above"),
    }
}

fn destroy_output(state: &mut State, output_global: u32) {
    let keys = state
        .surfaces
        .keys()
        .filter(|key| key.output_global == output_global)
        .copied()
        .collect::<Vec<_>>();
    for key in keys {
        destroy_surface(state, key);
    }

    if let Some(output) = state.outputs.remove(&output_global) {
        if output.advertised_version >= 3 {
            output.proxy.release();
        }
        eprintln!("surface-topology-poc: removed output global={output_global}");
    }
}

fn destroy_surface(state: &mut State, key: SurfaceKey) {
    if let Some(surface) = state.surfaces.remove(&key) {
        surface.layer_surface.destroy();
        surface.surface.destroy();
        eprintln!(
            "surface-topology-poc: destroyed output={} role={:?}",
            key.output_global, key.role
        );
    }
}

fn shutdown(state: &mut State) {
    let keys = state.surfaces.keys().copied().collect::<Vec<_>>();
    for key in keys {
        destroy_surface(state, key);
    }
    for buffer in state.buffers.drain().map(|(_, buffer)| buffer) {
        buffer.destroy();
    }
    for output in state.outputs.drain().map(|(_, output)| output) {
        if output.advertised_version >= 3 {
            output.proxy.release();
        }
    }
}

fn protocol_layer(role: SurfaceRole) -> zwlr_layer_shell_v1::Layer {
    match role {
        SurfaceRole::Panel => zwlr_layer_shell_v1::Layer::Top,
        SurfaceRole::Unified | SurfaceRole::Notch | SurfaceRole::Overlay => {
            zwlr_layer_shell_v1::Layer::Overlay
        }
    }
}

fn allocate_buffer(
    state: &mut State,
    queue_handle: &QueueHandle<State>,
    key: SurfaceKey,
    width: u32,
    height: u32,
) -> Result<u32, Box<dyn Error>> {
    let buffer_id = state.next_buffer_id;
    state.next_buffer_id = state
        .next_buffer_id
        .checked_add(1)
        .ok_or("surface-topology-poc buffer id overflow")?;
    let stride = width
        .checked_mul(4)
        .ok_or("surface-topology-poc stride overflow")?;
    let byte_len = u64::from(stride)
        .checked_mul(u64::from(height))
        .ok_or("surface-topology-poc buffer size overflow")?;
    let pool_size = i32::try_from(byte_len).map_err(|_| "surface-topology-poc buffer too large")?;
    let file = tempfile::tempfile()?;
    file.set_len(byte_len)?;
    let pool = state
        .shm
        .create_pool(file.as_fd(), pool_size, queue_handle, ());
    let buffer = pool.create_buffer(
        0,
        i32::try_from(width).map_err(|_| "surface-topology-poc width too large")?,
        i32::try_from(height).map_err(|_| "surface-topology-poc height too large")?,
        i32::try_from(stride).map_err(|_| "surface-topology-poc stride too large")?,
        wl_shm::Format::Argb8888,
        queue_handle,
        buffer_id,
    );
    pool.destroy();
    state.buffers.insert(buffer_id, buffer);
    state.buffer_owners.insert(buffer_id, key);
    Ok(buffer_id)
}

struct Options {
    topology: SurfaceTopology,
    click_through: bool,
    exclusive_zone: bool,
}

impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let mut options = Self {
            topology: SurfaceTopology::Independent,
            click_through: false,
            exclusive_zone: true,
        };
        for argument in args {
            match argument.as_str() {
                "--topology=unified" => options.topology = SurfaceTopology::Unified,
                "--topology=independent" => options.topology = SurfaceTopology::Independent,
                "--click-through" => options.click_through = true,
                "--no-exclusive-zone" => options.exclusive_zone = false,
                "--help" | "-h" => {
                    println!(
                        "usage: surface-topology-poc [--topology=unified|independent] [--click-through] [--no-exclusive-zone]"
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
            } if interface == "wl_output" => bind_output(
                registry,
                queue_handle,
                state,
                &Global {
                    name,
                    interface,
                    version,
                },
            ),
            wl_registry::Event::GlobalRemove { name } => destroy_output(state, name),
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
        queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            wl_output::Event::Name { name } => {
                if let Some(output) = state.outputs.get_mut(&data.global_name) {
                    output.name = Some(name.clone());
                }
                ensure_output_surfaces(state, data.global_name, queue_handle);
            }
            wl_output::Event::Done => ensure_output_surfaces(state, data.global_name, queue_handle),
            _ => {}
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
            if let Some(buffer) = state.buffers.remove(data) {
                buffer.destroy();
            } else {
                buffer.destroy();
            }
            if let Some(key) = state.buffer_owners.remove(data)
                && let Some(surface) = state.surfaces.get_mut(&key)
                && surface.buffer_id == Some(*data)
            {
                surface.buffer_id = None;
            }
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, SurfaceKey> for State {
    fn event(
        state: &mut Self,
        layer_surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        key: &SurfaceKey,
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
                let Some(surface) = state.surfaces.get(key) else {
                    return;
                };
                configure_input_region(state, &surface.surface, *key, width, height, queue_handle);
                let buffer_id = match allocate_buffer(state, queue_handle, *key, width, height) {
                    Ok(buffer_id) => buffer_id,
                    Err(error) => {
                        eprintln!("surface-topology-poc: buffer allocation failed: {error}");
                        state.running = false;
                        return;
                    }
                };
                let buffer = state.buffers[&buffer_id].clone();
                let Some(surface) = state.surfaces.get_mut(key) else {
                    return;
                };
                surface.surface.attach(Some(&buffer), 0, 0);
                surface.surface.damage_buffer(
                    0,
                    0,
                    width.min(i32::MAX as u32) as i32,
                    height.min(i32::MAX as u32) as i32,
                );
                surface.surface.commit();
                surface.buffer_id = Some(buffer_id);
            }
            zwlr_layer_surface_v1::Event::Closed => {
                eprintln!(
                    "surface-topology-poc: compositor closed output={} role={:?}",
                    key.output_global, key.role
                );
                let recreate = state.outputs.contains_key(&key.output_global);
                destroy_surface(state, *key);
                if recreate {
                    ensure_output_surfaces(state, key.output_global, queue_handle);
                }
            }
            _ => {}
        }
    }
}

delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_region::WlRegion);
delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_surface::WlSurface);
delegate_noop!(State: ignore zwlr_layer_shell_v1::ZwlrLayerShellV1);
