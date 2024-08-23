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
use glazier::{
    kurbo::{Affine, Size},
    Application, HotKey, IdleToken, Menu, PointerEvent, Region, Scalable, SysMods, WinHandler,
    WindowBuilder, WindowHandle,
};
use std::{
    any::Any,
    num::NonZeroUsize,
    time::{Instant, SystemTime},
};
use vello::{
    peniko::Color,
    util::{RenderContext, RenderSurface},
    AaSupport, RenderParams, Renderer, RendererOptions, Scene,
};

use crate::EpgiGlazierSchedulerExtension;

pub struct AppLauncher {
    title: String,
    app: ArcBoxWidget,
}
const QUIT_MENU_ID: u32 = 0x100;

impl AppLauncher {
    pub fn new(app: ArcBoxWidget) -> Self {
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
        let glazier_app = Application::new().unwrap();
        let mut file_menu = Menu::new();
        file_menu.add_item(
            QUIT_MENU_ID,
            "E&xit",
            Some(&HotKey::new(SysMods::Cmd, "q")),
            Some(false),
            true,
        );
        let mut menubar = Menu::new();
        menubar.add_dropdown(Menu::new(), "Application", true);
        menubar.add_dropdown(file_menu, "&File", true);

        let mut main_state = MainState::new();
        main_state.start_scheduler_with(self.app);

        let window = WindowBuilder::new(glazier_app.clone())
            .handler(Box::new(main_state))
            .title(self.title)
            .menu(menubar)
            .size(Size::new(1024., 768.))
            .build()
            .unwrap();
        window.show();
        glazier_app.run(None);
    }
}

struct MainState {
    handle: WindowHandle,
    // app: App<T, V>,
    render_cx: RenderContext,
    surface: Option<RenderSurface<'static>>,
    renderer: Option<Renderer>,
    // root_layer: Option<Layer<Affine2dCanvas>>,
    scene: Scene,
    counter: u64,

    scheduler_join_handle: Option<std::thread::JoinHandle<()>>,
    frame_binding: Arc<SyncMutex<Option<SetState<FrameInfo>>>>,
    constraints_binding: Arc<SyncMutex<Option<SetState<BoxConstraints>>>>,
}

impl WinHandler for MainState {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
        // self.app.connect(handle.clone());
    }

    fn prepare_paint(&mut self) {}

    fn paint(&mut self, _: &Region) {
        println!("paint");
        self.render();
        self.schedule_render();
    }

    fn idle(&mut self, _: IdleToken) {}

    fn command(&mut self, id: u32) {
        match id {
            QUIT_MENU_ID => {
                self.handle.close();
                Application::global().quit()
            }
            _ => println!("unexpected id {}", id),
        }
    }

    fn accesskit_tree(&mut self) -> accesskit::TreeUpdate {
        todo!()
        // self.app.accesskit_connected = true;
        // self.app.accessibility()
    }

    fn accesskit_action(&mut self, request: accesskit::ActionRequest) {
        // todo!()
        // self.app
        //     .window_event(Event::TargetedAccessibilityAction(request));
        // self.handle.invalidate();
    }

    fn pointer_down(&mut self, event: &PointerEvent) {
        // todo!()
        // self.app.window_event(Event::MouseDown(event.into()));
        // self.handle.invalidate();
    }

    fn pointer_up(&mut self, event: &PointerEvent) {
        // todo!()
        // self.app.window_event(Event::MouseUp(event.into()));
        // self.handle.invalidate();
    }

    fn pointer_move(&mut self, event: &PointerEvent) {
        // todo!()
        // self.app.window_event(Event::MouseMove(event.into()));
        // self.handle.invalidate();
        // self.handle.set_cursor(&Cursor::Arrow);
    }

    fn wheel(&mut self, event: &PointerEvent) {
        // todo!()
        // self.app.window_event(Event::MouseWheel(event.into()));
        // self.handle.invalidate();
    }

    fn pointer_leave(&mut self) {
        // todo!()
        // self.app.window_event(Event::MouseLeft());
        // self.handle.invalidate();
    }

    fn size(&mut self, size: Size) {
        self.update_size(size)
    }

    fn request_close(&mut self) {
        self.handle.close();
    }

    fn destroy(&mut self) {
        Application::global().quit()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl MainState {
    fn new() -> Self {
        let render_cx = RenderContext::new().unwrap();

        Self {
            handle: Default::default(),
            render_cx,
            surface: None,
            renderer: None,
            scene: Default::default(),
            counter: 0,
            scheduler_join_handle: None,
            frame_binding: Default::default(),
            constraints_binding: Default::default(),
        }
    }

    #[cfg(target_os = "macos")]
    fn schedule_render(&self) {
        self.handle
            .get_idle_handle()
            .unwrap()
            .schedule_idle(IdleToken::new(0));
    }

    #[cfg(not(target_os = "macos"))]
    fn schedule_render(&self) {
        self.handle.invalidate();
    }

    fn update_size(&self, size_dp: Size) {
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
        let handle = &self.handle;
        let size_dp = handle.get_size();
        let insets_dp = handle.content_insets();
        let constraints = BoxConstraints {
            min_width: size_dp.width as f32,
            max_width: size_dp.width as f32,
            min_height: size_dp.height as f32,
            max_height: size_dp.height as f32,
        };
        let scheduler = get_current_scheduler();

        self.update_size(size_dp);
        self.update_frame(self.counter);
        let frame_results = scheduler.request_new_frame().recv().unwrap();
        let encoding = frame_results
            .composited
            .as_ref()
            .downcast_ref::<Arc<Affine2dEncoding>>()
            .unwrap();

        let scale = handle.get_scale().unwrap_or_default();
        let insets = insets_dp.to_px(scale);
        let mut size = size_dp.to_px(scale);
        size.width -= insets.x_value();
        size.height -= insets.y_value();
        let width = size.width as u32;
        let height = size.height as u32;
        if self.surface.is_none() {
            //println!("render size: {:?}", size);
            self.surface = Some(
                futures::executor::block_on(self.render_cx.create_surface(handle, width, height))
                    .unwrap(),
            );
        }
        if let Some(surface) = self.surface.as_mut() {
            if surface.config.width != width || surface.config.height != height {
                self.render_cx.resize_surface(surface, width, height);
            }
            let (scale_x, scale_y) = (scale.x(), scale.y());
            let transform = if scale_x != 1.0 || scale_y != 1.0 {
                Some(Affine::scale_non_uniform(scale_x, scale_y))
            } else {
                None
            };
            // let mut builder = SceneBuilder::for_scene(&mut self.scene);
            // builder.append(&encoding, transform);
            let mut scene = vello_encoding::Encoding::new();
            scene.reset();
            scene.append(
                &encoding,
                &transform.map(|transform| vello_encoding::Transform::from_kurbo(&transform)),
            );
            // SceneBuilder's API is crippled, we use an unsafe transmute to avoid invent a whole new set of APIs
            self.scene = unsafe { std::mem::transmute(scene) };

            self.counter += 1;
            let surface_texture = surface
                .surface
                .get_current_texture()
                .expect("failed to acquire next swapchain texture");
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
            surface_texture.present();
            device.poll(wgpu::Maintain::Wait);
        }
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

impl MainState {
    fn start_scheduler_with(&mut self, app: ArcBoxWidget) {
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

        let lane_scheduler = BuildScheduler::new(element_node, get_current_scheduler());

        let scheduler = Scheduler::new(lane_scheduler, EpgiGlazierSchedulerExtension::new());
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
            let (child, _) = ctx.use_state::<Option<ArcBoxWidget>>(None);
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
                Box::new(StateHook::<Option<ArcBoxWidget>> { val: None }) as _,
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
    let widget_binding = SetState::<Option<ArcBoxWidget>>::new(
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
    child: ArcBoxWidget,
) -> (
    ArcBoxWidget,
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
    child: ArcBoxWidget,
) -> (
    ArcBoxWidget,
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
