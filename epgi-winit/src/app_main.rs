use epgi_2d::{
    Affine2dEncoding, BoxConstraints, BoxProtocol, BoxProvider, BoxSize, RenderRoot, RootElement,
    RootView,
};
use epgi_common::ConstrainedBox;
use epgi_core::{
    foundation::{Arc, SyncMutex},
    hooks::{SetState, StateHook},
    nodes::Function,
    scheduler::{
        get_current_scheduler, setup_scheduler, BuildScheduler, Scheduler, SchedulerHandle,
    },
    tree::{create_root_element, ArcChildWidget, ChildWidget, ElementNode, Hooks, RenderObject},
};
use std::{
    any::Any,
    num::NonZeroUsize,
    time::{Instant, SystemTime},
};
use vello::{
    kurbo::Affine,
    peniko::Color,
    util::{RenderContext, RenderSurface},
    AaSupport, RenderParams, Renderer, RendererOptions, Scene,
};
use winit::{
    event::{ElementState, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::EpgiGlazierSchedulerExtension;

pub struct AppLauncher {
    title: String,
    app: ArcChildWidget<BoxProtocol>,
}

struct MainState<'a> {
    window: Arc<Window>,
    // app: App<T, V>,
    render_cx: RenderContext,
    surface: RenderSurface<'a>,
    renderer: Option<Renderer>,
    // root_layer: Option<Layer<Affine2dCanvas>>,
    scene: Scene,
    counter: u64,

    scheduler_join_handle: Option<std::thread::JoinHandle<()>>,
    frame_binding: Arc<SyncMutex<Option<SetState<FrameInfo>>>>,
    constraints_binding: Arc<SyncMutex<Option<SetState<BoxConstraints>>>>,
}

impl AppLauncher {
    pub fn new(app: ArcChildWidget<BoxProtocol>) -> Self {
        Self {
            title: "epgi app".into(),
            app,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn run(self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Wait);
        // let _guard = self.app.rt.enter();
        let window = WindowBuilder::new()
            .with_inner_size(winit::dpi::LogicalSize {
                width: 1024.,
                height: 768.,
            })
            .build(&event_loop)
            .unwrap();
        let mut main_state = MainState::new(window);
        main_state.start_scheduler_with(self.app);

        event_loop
            .run(move |event, elwt| {
                if let winit::event::Event::WindowEvent { event: e, .. } = event {
                    match e {
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::RedrawRequested => main_state.render(),
                        WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }) => {
                            // main_state.size(Size {
                            //     width: width.into(),
                            //     height: height.into(),
                            // });
                        }
                        WindowEvent::ModifiersChanged(modifiers) => {}
                        WindowEvent::CursorMoved {
                            position: winit::dpi::PhysicalPosition { x, y },
                            ..
                        } => {}
                        WindowEvent::CursorLeft { .. } => {}
                        WindowEvent::MouseInput { state, button, .. } => {}
                        WindowEvent::MouseWheel { delta, .. } => {}
                        _ => (),
                    }
                }
            })
            .unwrap();
    }
}

impl<'a> MainState<'a> {
    fn new(window: Window) -> Self {
        let mut render_cx = RenderContext::new().unwrap();
        let size = window.inner_size();
        let window = Arc::new(window);
        let surface = futures::executor::block_on(render_cx.create_surface(
            window.clone(),
            size.width,
            size.height,
        ))
        .unwrap();
        MainState {
            window,
            render_cx,
            surface,
            renderer: None,
            scene: Scene::default(),
            counter: 0,
            scheduler_join_handle: None,
            frame_binding: Default::default(),
            constraints_binding: Default::default(),
        }
    }

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

    fn update_frame(&self, counter: u64) {
        if let Some(set_frame) = &*self.frame_binding.lock() {
            get_current_scheduler().create_sync_job(|job_builder| {
                set_frame.set(FrameInfo::now(self.counter), job_builder);
            });
        }
    }

    fn render(&mut self) {
        // let fragment = self.app.fragment();
        let scale = self.window.scale_factor();
        let size = self.window.inner_size();
        let width = size.width;
        let height = size.height;

        let scheduler = get_current_scheduler();
        self.update_frame(self.counter);

        let frame_results = scheduler.request_new_frame().recv().unwrap();
        let encoding = frame_results
            .composited
            .as_ref()
            .downcast_ref::<Arc<Affine2dEncoding>>()
            .unwrap();

        if self.surface.config.width != width || self.surface.config.height != height {
            self.render_cx
                .resize_surface(&mut self.surface, width, height);
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
        let surface_texture = self
            .surface
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let dev_id = self.surface.dev_id;
        let device = &self.render_cx.devices[dev_id].device;
        let queue = &self.render_cx.devices[dev_id].queue;
        let renderer_options = RendererOptions {
            surface_format: Some(self.surface.format),
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
        surface_texture.present();
        device.poll(wgpu::Maintain::Wait);
    }
}

#[derive(Debug, Clone)]
struct FrameInfo {
    instant: Instant,
    system_time: SystemTime,
    frame_count: u64,
}

impl FrameInfo {
    pub fn now(frame_count: u64) -> Self {
        Self {
            instant: Instant::now(),
            system_time: SystemTime::now(),
            frame_count,
        }
    }
}

impl<'a> MainState<'a> {
    fn start_scheduler_with(&mut self, app: ArcChildWidget<BoxProtocol>) {
        // First we construct an empty root with no children. Later we will inject our application widget inside
        let (element_node, render_object, widget_binding) = initialize_root();

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

        initialize_scheduler_handle();

        // Now we call the scheduler to inject the wrapped widget
        get_current_scheduler().create_sync_job(|job_builder| {
            widget_binding.set(Some(child), job_builder);
        });

        let build_scheduler = BuildScheduler::new(element_node, get_current_scheduler());

        let scheduler = Scheduler::new(build_scheduler, EpgiGlazierSchedulerExtension::new());
        let join_handle = std::thread::spawn(move || {
            scheduler.start_event_loop(get_current_scheduler());
        });

        self.scheduler_join_handle = Some(join_handle);
    }
}

fn initialize_root() -> (
    Arc<ElementNode<RootElement>>,
    Arc<RenderObject<RenderRoot>>,
    SetState<Option<Arc<dyn ChildWidget<BoxProtocol>>>>,
) {
    // First we inflate a simple root widget by hand, with no children attached.
    // This is to allow the injection of a child widget later on.
    let root_widget = Arc::new(RootView {
        build: Box::new(move |mut ctx| {
            let (child, _) = ctx.use_state::<Option<ArcChildWidget<BoxProtocol>>>(None);
            child
        }),
    });
    let element = RootElement {};
    let (element_node, render_object) = create_root_element::<RootElement, RenderRoot>(
        root_widget,
        element,
        None,
        RenderRoot { child: None },
        None,
        Hooks {
            array_hooks: [
                Box::new(StateHook::<Option<ArcChildWidget<BoxProtocol>>> { val: None }) as _,
            ]
            .into(),
        },
        BoxConstraints::default(),
        BoxSize {
            width: f32::INFINITY,
            height: f32::INFINITY,
        },
        (),
    );

    // Construct the widget injection binding by hand for later use.
    let widget_binding = SetState::<Option<ArcChildWidget<BoxProtocol>>>::new(
        Arc::downgrade(&element_node.context),
        0,
    );

    (element_node, render_object, widget_binding)
}

fn initialize_scheduler_handle() {
    let sync_threadpool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let async_threadpool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let scheduler_handle = SchedulerHandle::new(sync_threadpool, async_threadpool);
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

    let child = Arc::new(Function(move |mut ctx| {
        let frame_binding = frame_binding.clone();
        let child = child.clone();
        let (frame, set_frame) = ctx.use_state_with(|| FrameInfo::now(0));
        ctx.use_effect(move || *frame_binding.lock() = Some(set_frame));
        BoxProvider::value_inner(
            frame.frame_count,
            BoxProvider::value_inner(
                frame.system_time,
                BoxProvider::value_inner(frame.instant, child),
            ),
        )
    }));
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

    let child = Arc::new(Function(move |mut ctx| {
        let constraints_binding = constraints_binding.clone();
        let child = child.clone();
        let (constraints, set_constraints) = ctx.use_state_with_default::<BoxConstraints>();
        ctx.use_effect(move || *constraints_binding.lock() = Some(set_constraints));
        Arc::new(ConstrainedBox { constraints, child })
    }));
    (child, result)
}