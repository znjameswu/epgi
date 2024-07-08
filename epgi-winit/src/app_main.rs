use epgi_2d::{Affine2dEncoding, BoxConstraints, BoxOffset, BoxProtocol, BoxSize, RootView};
use epgi_common::{ConstrainedBox, FrameInfo, PointerEvent};
use epgi_core::{
    foundation::{unbounded_channel_sync, Arc, Asc, SyncMpscReceiver, SyncMutex},
    hooks::SetState,
    nodes::Builder,
    scheduler::{get_current_scheduler, setup_scheduler, FrameMetrics, Scheduler, SchedulerHandle},
    tree::{ArcChildWidget, LayoutResults},
    Provider,
};
use std::{num::NonZeroUsize, sync::atomic::Ordering, time::Instant};
use tracing::subscriber::SetGlobalDefaultError;
use typed_builder::TypedBuilder;
use vello::{
    kurbo::Affine,
    peniko::Color,
    util::{RenderContext, RenderSurface},
    AaSupport, RenderParams, Renderer, RendererOptions, Scene,
};
use wgpu::PresentMode;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
};

pub use winit::window::{Window, WindowAttributes};

use crate::{
    EpgiGlazierSchedulerExtension, FrameStatSample, FrameStats, WinitPointerEventConverter,
};

pub enum WindowState<'a> {
    Uninitialized(WindowAttributes),
    Rendering {
        window: Arc<Window>,
        surface: RenderSurface<'a>,
        // accesskit_adapter: Adapter,
    },
    Suspended {
        window: Arc<Window>,
        // accesskit_adapter: Adapter,
    },
}

struct MainState<'a> {
    window: WindowState<'a>,

    // app: App<T, V>,
    render_cx: RenderContext,
    renderer: Option<Renderer>,
    // root_layer: Option<Layer<Affine2dCanvas>>,
    scene: Scene,
    counter: u64,

    scheduler_join_handle: Option<std::thread::JoinHandle<()>>,
    frame_binding: Arc<SyncMutex<Option<SetState<FrameInfo>>>>,
    constraints_binding: Arc<SyncMutex<Option<SetState<BoxConstraints>>>>,
    pointer_event_converter: WinitPointerEventConverter,

    frame_stats: FrameStats,
    print_stats: bool,
}

#[derive(TypedBuilder)]
pub struct AppLauncher {
    app: ArcChildWidget<BoxProtocol>,
    #[builder(default, setter(strip_option))]
    sync_threadpool_builder: Option<rayon::ThreadPoolBuilder>,
    #[builder(default, setter(strip_option))]
    async_threadpool_builder: Option<rayon::ThreadPoolBuilder>,
    #[cfg(feature = "tokio")]
    #[builder(default, setter(strip_option))]
    tokio_handle: Option<tokio::runtime::Handle>,

    #[builder(default)]
    print_stats: bool,

    window: WindowAttributes,
    #[builder(default = EventLoop::new().unwrap(), setter(skip))]
    event_loop: EventLoop<()>,
}

impl AppLauncher {
    pub fn run(self) {
        pretty_env_logger::init();
        let render_cx = RenderContext::new();

        let (tx, rx) = unbounded_channel_sync();
        let mut main_state = MainState {
            window: WindowState::Uninitialized(self.window),
            render_cx,
            renderer: None,
            scene: Scene::default(),
            counter: 0,
            scheduler_join_handle: None,
            frame_binding: Default::default(),
            constraints_binding: Default::default(),
            pointer_event_converter: WinitPointerEventConverter::new(tx),

            frame_stats: FrameStats::new(),
            print_stats: self.print_stats,
        };

        #[cfg(feature = "tokio")]
        let tokio_handle = self.tokio_handle.unwrap_or_else(|| {
            tokio::runtime::Builder::new_multi_thread()
                .thread_name("tokio pool")
                .enable_time()
                .build()
                .unwrap()
                .handle()
                .clone()
        });

        let spawn_hook = ();

        #[cfg(feature = "tokio")]
        let spawn_hook = (spawn_hook, tokio_handle);

        let rayon_spawn_handler = |thread: rayon::ThreadBuilder| {
            // Adapted from rayon documentation
            let mut b = std::thread::Builder::new();
            if let Some(name) = thread.name() {
                b = b.name(name.to_owned());
            }
            if let Some(stack_size) = thread.stack_size() {
                b = b.stack_size(stack_size);
            }
            let spawn_hook = spawn_hook.clone();
            b.spawn(move || {
                let _guard = spawn_hook.enter();
                thread.run();
            })?;
            Ok(())
        };

        let async_threadpool = self
            .async_threadpool_builder
            .unwrap_or_else(|| {
                rayon::ThreadPoolBuilder::new()
                    .thread_name(|index| format!("epgi async pool {}", index))
            })
            .spawn_handler(rayon_spawn_handler)
            .build()
            .unwrap();
        let sync_threadpool = self
            .sync_threadpool_builder
            .unwrap_or_else(|| {
                rayon::ThreadPoolBuilder::new()
                    .thread_name(|index| format!("epgi sync pool {}", index))
            })
            .spawn_handler(rayon_spawn_handler)
            .build()
            .unwrap();

        // If there is no default tracing subscriber, we set our own. If one has
        // already been set, we get an error which we swallow.
        // By now, we're about to take control of the event loop. The user is unlikely
        // to try to set their own subscriber once the event loop has started.
        let _ = try_init_tracing();

        initialize_scheduler_handle(sync_threadpool, async_threadpool);
        main_state.start_scheduler_with(self.app, rx, spawn_hook);

        self.event_loop.run_app(&mut main_state).unwrap()
    }
}

pub trait SpawnHook: Clone + Send + 'static {
    type Guard<'a>
    where
        Self: 'a;

    fn enter(&self) -> Self::Guard<'_>;
}

impl<T1, T2> SpawnHook for (T1, T2)
where
    T1: SpawnHook,
    T2: SpawnHook,
{
    type Guard<'a> = (T1::Guard<'a> , T2::Guard<'a>)
    where
        Self: 'a;

    fn enter(&self) -> Self::Guard<'_> {
        (self.0.enter(), self.1.enter())
    }
}

impl<F, G> SpawnHook for F
where
    F: Fn() -> G + Clone + Send + 'static,
{
    type Guard<'a> = G
    where
        Self: 'a;

    fn enter(&self) -> Self::Guard<'_> {
        self()
    }
}

impl SpawnHook for () {
    type Guard<'a> = ()
    where
        Self: 'a;

    fn enter(&self) -> Self::Guard<'_> {}
}

#[cfg(feature = "tokio")]
impl SpawnHook for tokio::runtime::Handle {
    type Guard<'a> = tokio::runtime::EnterGuard<'a>
    where
        Self: 'a;

    fn enter(&self) -> Self::Guard<'_> {
        self.enter()
    }
}

impl ApplicationHandler for MainState<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match std::mem::replace(
            &mut self.window,
            // TODO: Is there a better default value which could be used?
            WindowState::Uninitialized(WindowAttributes::default()),
        ) {
            WindowState::Uninitialized(attributes) => {
                let visible = attributes.visible;
                let attributes = attributes.with_visible(false);

                let window = event_loop.create_window(attributes).unwrap();

                window.set_visible(visible);
                let window = Arc::new(window);
                let size = window.inner_size();
                let surface = futures::executor::block_on(self.render_cx.create_surface(
                    window.clone(),
                    size.width,
                    size.height,
                    PresentMode::AutoNoVsync,
                ))
                .unwrap();
                let scale_factor = window.scale_factor();
                self.window = WindowState::Rendering { window, surface };
                // self.render_root
                //     .handle_window_event(WindowEvent::Rescale(scale_factor));
            }
            WindowState::Suspended { window } => {
                let size = window.inner_size();
                let surface = futures::executor::block_on(self.render_cx.create_surface(
                    window.clone(),
                    size.width,
                    size.height,
                    PresentMode::AutoVsync,
                ))
                .unwrap();
                self.window = WindowState::Rendering { window, surface }
            }
            _ => {
                // We have received a redundant resumed event. That's allowed by winit
            }
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        match std::mem::replace(
            &mut self.window,
            // TODO: Is there a better default value which could be used?
            WindowState::Uninitialized(WindowAttributes::default()),
        ) {
            WindowState::Rendering { window, surface } => {
                drop(surface);
                self.window = WindowState::Suspended { window };
            }
            _ => {
                // We have received a redundant resumed event. That's allowed by winit
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let WindowState::Rendering { window, .. } = &mut self.window else {
            tracing::warn!(
                ?event,
                "Got window event whilst suspended or before window created"
            );
            return;
        };

        use WindowEvent::*;
        match event {
            CloseRequested => event_loop.exit(),
            RedrawRequested => {
                self.render();
            }
            Resized(winit::dpi::PhysicalSize { width, height }) => {
                self.update_size(BoxSize {
                    width: width as _,
                    height: height as _,
                });
            }
            ModifiersChanged(modifiers) => {}
            CursorMoved { .. }
            | CursorEntered { .. }
            | CursorLeft { .. }
            | MouseWheel { .. }
            | MouseInput { .. }
            | PinchGesture { .. }
            | DoubleTapGesture { .. }
            | RotationGesture { .. }
            | TouchpadPressure { .. }
            | AxisMotion { .. }
            | Touch { .. } => {
                self.pointer_event_converter.convert(&event);
                window.request_redraw();
            }
            _ => (),
        }

        self.handle_signals(event_loop)
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
        let WindowState::Rendering { window, .. } = &mut self.window else {
            tracing::warn!(
                ?event,
                "Got window event whilst suspended or before window created"
            );
            return;
        };

        self.handle_signals(event_loop)
    }
}

impl<'a> MainState<'a> {
    fn update_size(&self, size_dp: BoxSize) {
        let constraints = BoxConstraints {
            min_width: size_dp.width as f32,
            max_width: size_dp.width as f32,
            min_height: size_dp.height as f32,
            max_height: size_dp.height as f32,
        };
        if let Some(set_constraints) = &*self.constraints_binding.lock() {
            get_current_scheduler().create_sync_job(|job_builder| {
                set_constraints.set(constraints, job_builder);
            });
        }
    }

    fn render(&mut self) {
        let WindowState::Rendering {
            window, surface, ..
        } = &mut self.window
        else {
            tracing::warn!("Tried to render whilst suspended or before window created");
            return;
        };
        let scale = window.scale_factor();
        let size = window.inner_size();
        let width = size.width;
        let height = size.height;

        let scheduler = get_current_scheduler();
        // Update frame
        if let Some(set_frame) = &*self.frame_binding.lock() {
            scheduler.create_sync_job(|job_builder| {
                set_frame.set(FrameInfo::now(self.counter), job_builder);
            });
        }

        let frame_results = scheduler.request_new_frame().recv().unwrap();

        let raster_start_time = Instant::now();

        let encoding = frame_results
            .composited
            .as_ref()
            .downcast_ref::<Arc<Affine2dEncoding>>()
            .unwrap();

        if surface.config.width != width || surface.config.height != height {
            self.render_cx.resize_surface(surface, width, height);
        }
        let transform = if scale != 1.0 {
            Some(Affine::scale(scale))
        } else {
            None
        };

        let mut scene = vello_encoding::Encoding::new();
        scene.reset();
        scene.append(
            &encoding,
            &transform.map(|transform| vello_encoding::Transform::from_kurbo(&transform)),
        );
        // SceneBuilder's API is crippled, we use an unsafe transmute to avoid invent a whole new set of APIs
        self.scene = unsafe { std::mem::transmute(scene) };

        self.counter += 1;
        let Ok(surface_texture) = surface.surface.get_current_texture() else {
            tracing::warn!("failed to acquire next swapchain texture");
            return;
        };
        let dev_id = surface.dev_id;
        let device = &self.render_cx.devices[dev_id].device;
        let queue = &self.render_cx.devices[dev_id].queue;
        let renderer_options = RendererOptions {
            surface_format: Some(surface.format),
            use_cpu: false,
            antialiasing_support: AaSupport {
                area: true,
                msaa8: false,
                msaa16: false,
            },
            num_init_threads: NonZeroUsize::new(1),
        };
        let render_params = RenderParams {
            base_color: Color::BLACK,
            width,
            height,
            antialiasing_method: vello::AaConfig::Area,
        };
        self.renderer
            .get_or_insert_with(|| Renderer::new(device, renderer_options).unwrap())
            .render_to_surface(device, queue, &self.scene, &surface_texture, &render_params)
            .expect("failed to render to surface");

        let raster_time = Instant::now().duration_since(raster_start_time).as_micros() as u64;
        surface_texture.present();
        device.poll(wgpu::Maintain::Wait);

        self.frame_stats.add_sample(FrameStatSample {
            timestamp: Instant::now(),
            ui_metrics: frame_results.metrics.clone(),
            raster_time,
        });
        if self.print_stats {
            let metrics = frame_results.metrics;
            #[allow(unused_variables)]
            let FrameMetrics {
                build_time,
                sync_build_time,
                layout_time,
                paint_time,
                composite_time,
                ..
            } = metrics;
            #[allow(unused_variables)]
            let FrameStats {
                frame_count,
                build_time_sum,
                layout_time_sum,
                paint_time_sum,
                raster_time_sum,
                frame_time_sum,
                build_time_low,
                layout_time_low,
                paint_time_low,
                raster_time_low,
                frame_time_ms_low,
                ..
            } = self.frame_stats;
            println!(
                "Frame {:5} built with: UI{:>5.1} ms, raster{:>5.1} ms, build{:>5.1}/{:>5.1}/{:>5.1} ms, build+layout{:>5.1}/{:>5.1}/{:>5.1} ms, paint{:>5.1}/{:>5.1}/{:>5.1} ms. Avg FPS:{:>5.1}/{:>5.1}/{:>5.1}",
                frame_count,
                metrics.frame_time() as f32 / 1000.0,
                raster_time as f32 / 1000.0,
                build_time as f32 / 1000.0,
                build_time_sum as f32 / frame_count as f32 / 1000.0,
                build_time_low as f32 /1000.0,
                (build_time + layout_time) as f32 / 1000.0,
                (build_time_sum + layout_time_sum) as f32 / frame_count as f32 / 1000.0,
                (build_time_low + layout_time_low) as f32 / 1000.0,
                paint_time as f32 / 1000.0,
                paint_time_sum as f32 / frame_count as f32 / 1000.0,
                paint_time_low as f32 /1000.0,
                1000.0 / self.frame_stats.get_frame_time_ms_avg().unwrap_or(-1.0),
                1000000.0 / frame_time_sum as f32 * (frame_count - 1) as f32,
                1000.0 / frame_time_ms_low
                // self.frame_stats.get_raster_time_ms_avg().unwrap_or(-1.0),
                // self.frame_stats.get_ui_time_ms_avg().unwrap_or(-1.0),
            )
        }
    }

    fn handle_signals(&mut self, _event_loop: &ActiveEventLoop) {
        let WindowState::Rendering { window, .. } = &mut self.window else {
            tracing::warn!("Tried to handle a signal whilst suspended or before window created");
            return;
        };
        if get_current_scheduler()
            .request_redraw
            .swap(false, Ordering::Acquire)
        {
            window.request_redraw()
        }
    }
}

impl<'a> MainState<'a> {
    fn start_scheduler_with(
        &mut self,
        app: ArcChildWidget<BoxProtocol>,
        rx: SyncMpscReceiver<PointerEvent>,
        spawn_hook: impl SpawnHook,
    ) {
        // Now we wrap the application in wrapper widgets that provides bindigns to basic functionalities,
        // such as window size and frame information.
        //
        // The whole point of these wrapper widgets is to provide corresponding bindings to the embedding to pump events into.
        //
        // The wrapping is done progressively from inside out.
        // This method is to circumvent Rust's limitation on closure captures.
        // There is no way of directly sending a value to inner closure without it first being captured by the outer closure,
        // Which would lead to the accumulation of variable captures at the outmost closure.
        //
        // The most frequently updated binding should comes in the innermost wrapper.

        let child = app;

        let (child, frame_binding) = bind_frame_info(child);
        self.frame_binding = frame_binding;

        let (child, constraints_binding) = bind_constraints(child);
        self.constraints_binding = constraints_binding;

        let scheduler = Scheduler::new(
            Asc::new(RootView { child }),
            LayoutResults::new(BoxConstraints::default(), BoxSize::INFINITY, ()),
            BoxOffset::ZERO,
            get_current_scheduler(),
            EpgiGlazierSchedulerExtension::new(rx),
        );
        let join_handle = std::thread::Builder::new()
            .name("epgi scheduler".into())
            .spawn(move || {
                let _guard = spawn_hook.enter();
                get_current_scheduler().sync_threadpool.install(|| {});
                scheduler.start_event_loop(get_current_scheduler());
            })
            .unwrap();

        self.scheduler_join_handle = Some(join_handle);
    }
}

fn initialize_scheduler_handle(
    sync_threadpool_builder: rayon::ThreadPool,
    async_threadpool_builder: rayon::ThreadPool,
) {
    // let sync_threadpool_builder = rayon::ThreadPoolBuilder::new()
    //     .num_threads(1)
    //     .build()
    //     .unwrap();
    // let async_threadpool_builder = rayon::ThreadPoolBuilder::new()
    //     .num_threads(1)
    //     .build()
    //     .unwrap();
    // #[cfg(tokio)]
    // {
    //     let tokio_rt = tokio::runtime::Builder::new_multi_thread()
    //         .worker_threads(1)
    //         .build()
    //         .unwrap();
    //     let tokio_handle = tokio_rt.handle();
    //     // sync_threadpool_builder.broadcast(|_| )
    // }
    let scheduler_handle = SchedulerHandle::new(sync_threadpool_builder, async_threadpool_builder);
    unsafe {
        setup_scheduler(scheduler_handle);
    }
}

fn bind_frame_info(
    child: ArcChildWidget<BoxProtocol>,
) -> (
    ArcChildWidget<BoxProtocol>,
    Arc<SyncMutex<Option<SetState<FrameInfo>>>>,
) {
    // Bind the frame info, which provides time.
    let frame_binding = Arc::new(SyncMutex::<Option<SetState<FrameInfo>>>::new(None));
    let result = frame_binding.clone();

    let child = Arc::new(Builder {
        builder: move |ctx| {
            let frame_binding = frame_binding.clone();
            let child = child.clone();
            let (frame, set_frame) = ctx.use_state_with(|| FrameInfo::now(0));
            ctx.use_effect(move |_| *frame_binding.lock() = Some(set_frame), ());
            Provider!(value = frame, child)
        },
    });
    (child, result)
}

fn bind_constraints(
    child: ArcChildWidget<BoxProtocol>,
) -> (
    ArcChildWidget<BoxProtocol>,
    Arc<SyncMutex<Option<SetState<BoxConstraints>>>>,
) {
    // Bind the window size.
    let constraints_binding = Arc::new(SyncMutex::<Option<SetState<BoxConstraints>>>::new(None));
    let result = constraints_binding.clone();

    let child = Arc::new(Builder {
        builder: move |ctx| {
            let constraints_binding = constraints_binding.clone();
            let child = child.clone();
            let (constraints, set_constraints) = ctx.use_state_default::<BoxConstraints>();
            ctx.use_effect_nodep(move || *constraints_binding.lock() = Some(set_constraints));
            Arc::new(ConstrainedBox {
                constraints: constraints.clone(),
                child,
            })
        },
    });
    (child, result)
}

pub(crate) fn try_init_tracing() -> Result<(), SetGlobalDefaultError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use time::macros::format_description;
        use tracing_subscriber::filter::LevelFilter;
        use tracing_subscriber::fmt::time::UtcTime;
        use tracing_subscriber::prelude::*;
        use tracing_subscriber::EnvFilter;

        // Default level is DEBUG in --dev, INFO in --release
        // DEBUG should print a few logs per low-density event.
        // INFO should only print logs for noteworthy things.
        let default_level = if cfg!(debug_assertions) {
            LevelFilter::DEBUG
        } else {
            LevelFilter::INFO
        };
        // Use EnvFilter to allow the user to override the log level without recompiling.
        // TODO - Print error message if the env var is incorrectly formatted.
        let env_filter = EnvFilter::builder()
            .with_default_directive(default_level.into())
            .with_env_var("RUST_LOG")
            .from_env_lossy();
        // This format is more concise than even the 'Compact' default:
        // - We print the time without the date (GUI apps usually run for very short periods).
        // - We print the time with seconds precision (we really don't need anything lower).
        // - We skip the target. In app code, the target is almost always visual noise. By
        //   default, it only gives you the module a log was defined in. This is rarely useful;
        //   the log message is much more helpful for finding a log's location.
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_timer(UtcTime::new(format_description!(
                "[hour]:[minute]:[second]"
            )))
            .with_target(false);

        let registry = tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer);
        tracing::dispatcher::set_global_default(registry.into())
    }

    // Note - tracing-wasm might not work in headless Node.js. Probably doesn't matter anyway,
    // because this is a GUI framework, so wasm targets will virtually always be browsers.
    #[cfg(target_arch = "wasm32")]
    {
        // Ignored if the panic hook is already set
        console_error_panic_hook::set_once();

        let max_level = if cfg!(debug_assertions) {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        };
        let config = tracing_wasm::WASMLayerConfigBuilder::new()
            .set_max_level(max_level)
            .build();

        tracing::subscriber::set_global_default(
            Registry::default().with(tracing_wasm::WASMLayer::new(config)),
        )
    }
}
