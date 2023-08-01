/// Modified based on xilem:src/app_main.rs
use std::any::Any;

use epgi_2d::Affine2dCanvas;
use glazier::{
    kurbo::{Affine, Size},
    text::Event,
    Application, Cursor, HotKey, IdleToken, Menu, MouseEvent, Region, Scalable, SysMods,
    WinHandler, WindowBuilder, WindowHandle,
};
use vello::{
    peniko::Color,
    util::{RenderContext, RenderSurface},
    RenderParams, Renderer, RendererOptions, Scene, SceneBuilder,
};

use epgi_core::{common::LayerScope, scheduler::get_current_scheduler};

const QUIT_MENU_ID: u32 = 0x100;

pub fn run() {
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
    let app = Application::new().unwrap();
    let mut builder = WindowBuilder::new(app.clone());
    // let _guard = self.app.rt.enter();
    let main_state = MainState::new();
    builder.set_handler(Box::new(main_state));
    builder.set_title("TODO");
    builder.set_menu(menubar);
    builder.set_size(Size::new(1024., 768.));
    let window = builder.build().unwrap();
    window.show();
    app.run(None);
}

struct MainState {
    handle: WindowHandle,
    // app: App<T, V>,
    render_cx: RenderContext,
    surface: Option<RenderSurface>,
    renderer: Option<Renderer>,
    // root_layer: Option<Layer<Affine2dCanvas>>,
    scene: Scene,
    counter: u64,
}

impl WinHandler for MainState {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
        todo!()
        // self.app.connect(handle.clone());
    }

    fn prepare_paint(&mut self) {}

    fn paint(&mut self, _: &Region) {
        let scheduler = get_current_scheduler();
        let new_frame_ready_listener = scheduler.new_frame_ready.listen();
        todo!();
        // self.app.paint();
        // self.render();
        // scheduler.
        new_frame_ready_listener.wait();
        // self.root_layer
        self.schedule_render();
    }

    // TODO: temporary hack
    fn idle(&mut self, _: IdleToken) {
        todo!();
        // self.app.paint();
        // self.render();
        // self.schedule_render();
    }

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
        todo!()
        // self.app
        //     .window_event(Event::TargetedAccessibilityAction(request));
        // self.handle.invalidate();
    }

    fn mouse_down(&mut self, event: &MouseEvent) {
        todo!()
        // self.app.window_event(Event::MouseDown(event.into()));
        // self.handle.invalidate();
    }

    fn mouse_up(&mut self, event: &MouseEvent) {
        todo!()
        // self.app.window_event(Event::MouseUp(event.into()));
        // self.handle.invalidate();
    }

    fn mouse_move(&mut self, event: &MouseEvent) {
        todo!()
        // self.app.window_event(Event::MouseMove(event.into()));
        // self.handle.invalidate();
        // self.handle.set_cursor(&Cursor::Arrow);
    }

    fn wheel(&mut self, event: &MouseEvent) {
        todo!()
        // self.app.window_event(Event::MouseWheel(event.into()));
        // self.handle.invalidate();
    }

    fn mouse_leave(&mut self) {
        todo!()
        // self.app.window_event(Event::MouseLeft());
        // self.handle.invalidate();
    }

    fn size(&mut self, size: Size) {
        todo!()
        // self.app.size(size);
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

    fn render(&mut self) {
        todo!();
        // let fragment = self.app.fragment();
        // let handle = &self.handle;
        // let scale = handle.get_scale().unwrap_or_default();
        // let insets = handle.content_insets().to_px(scale);
        // let mut size = handle.get_size().to_px(scale);
        // size.width -= insets.x_value();
        // size.height -= insets.y_value();
        // let width = size.width as u32;
        // let height = size.height as u32;
        // if self.surface.is_none() {
        //     //println!("render size: {:?}", size);
        //     self.surface = Some(futures::executor::block_on(
        //         self.render_cx.create_surface(handle, width, height),
        //     ));
        // }
        // if let Some(surface) = self.surface.as_mut() {
        //     if surface.config.width != width || surface.config.height != height {
        //         self.render_cx.resize_surface(surface, width, height);
        //     }
        //     let (scale_x, scale_y) = (scale.x(), scale.y());
        //     let transform = if scale_x != 1.0 || scale_y != 1.0 {
        //         Some(Affine::scale_non_uniform(scale_x, scale_y))
        //     } else {
        //         None
        //     };
        //     let mut builder = SceneBuilder::for_scene(&mut self.scene);
        //     builder.append(&fragment, transform);
        //     self.counter += 1;
        //     let surface_texture = surface
        //         .surface
        //         .get_current_texture()
        //         .expect("failed to acquire next swapchain texture");
        //     let dev_id = surface.dev_id;
        //     let device = &self.render_cx.devices[dev_id].device;
        //     let queue = &self.render_cx.devices[dev_id].queue;
        //     let renderer_options = RendererOptions {
        //         surface_format: Some(surface.format),
        //     };
        //     let render_params = RenderParams {
        //         base_color: Color::BLACK,
        //         width,
        //         height,
        //     };
        //     self.renderer
        //         .get_or_insert_with(|| Renderer::new(device, &renderer_options).unwrap())
        //         .render_to_surface(device, queue, &self.scene, &surface_texture, &render_params)
        //         .expect("failed to render to surface");
        //     surface_texture.present();
        //     device.poll(wgpu::Maintain::Wait);
        // }
    }
}
