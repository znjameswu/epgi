mod state;
use std::{
    sync::Arc,
    time::{Instant, SystemTime},
};

use epgi_2d::{BoxConstraints, BoxProtocol, RenderRootView, RootView, RootViewElement};
use epgi_common::ConstrainedBox;
use epgi_core::{
    tree::{create_root_element, ArcChildWidget, Element, Function, Hooks, ReconcileItem},
    foundation::SyncMutex,
    hooks::{SetState, StateHook},
    scheduler::{
        get_current_scheduler, setup_scheduler, Scheduler, SchedulerHandle, TreeScheduler,
    },
};
pub use state::*;

#[derive(Debug, Clone)]
struct FrameInfo {
    instant: Instant,
    system_time: SystemTime,
    frame_count: usize,
}

impl FrameInfo {
    pub fn now() -> Self {
        Self {
            instant: Instant::now(),
            system_time: SystemTime::now(),
            frame_count: 0,
        }
    }
}

pub fn run_app(app: ArcChildWidget<BoxProtocol>) {
    // First we inflate a simple root widget by hand, with no children attached.
    // This is to allow the injection of a child widget later on.
    let root_widget = Arc::new(RootView {
        build: Box::new(move |mut ctx| {
            let (child, _) = ctx.use_state::<Option<ArcChildWidget<BoxProtocol>>>(None);
            child
        }),
    });
    let element = RootViewElement { child: None };
    let element_node = create_root_element::<RenderRootView>(
        root_widget,
        element,
        Hooks {
            array_hooks: [
                Box::new(StateHook::<Option<ArcChildWidget<BoxProtocol>>> { val: None }) as _,
            ]
            .into(),
        },
        BoxConstraints::default(),
    );

    // Construct the widget injection binding by hand for later use.
    let widget_binding = SetState::<Option<ArcChildWidget<BoxProtocol>>>::new(
        Arc::downgrade(&element_node.context),
        0,
    );

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

    // Bind the frame info, which provides time.
    let frame_binding = Arc::new(SyncMutex::<Option<SetState<FrameInfo>>>::new(None));

    let child = Arc::new(Function(move |mut ctx| {
        let frame_binding = frame_binding.clone();
        let child = child.clone();
        let (frame, set_frame) = ctx.use_state_with(FrameInfo::now);
        ctx.use_effect(move || *frame_binding.lock() = Some(set_frame));
        child
    }));

    // Bind the window size.
    let constraints_binding = Arc::new(SyncMutex::<Option<SetState<BoxConstraints>>>::new(None));

    let child = Arc::new(Function(move |mut ctx| {
        let constraints_binding = constraints_binding.clone();
        let child = child.clone();
        let (constraints, set_constraints) = ctx.use_state_with_default::<BoxConstraints>();
        ctx.use_effect(move || *constraints_binding.lock() = Some(set_constraints));
        Arc::new(ConstrainedBox { constraints, child })
    }));

    let tree_scheduler = TreeScheduler::new(element_node.clone());
    let sync_threadpool = rayon::ThreadPoolBuilder::new().build().unwrap();
    let async_threadpool = rayon::ThreadPoolBuilder::new().build().unwrap();
    let scheduler_handle = SchedulerHandle::new(sync_threadpool, async_threadpool);
    unsafe {
        setup_scheduler(scheduler_handle);
    }

    let scheduler = Scheduler::new(tree_scheduler);
    let join_handle = std::thread::spawn(move || {
        scheduler.start_event_loop(get_current_scheduler());
    });

    // Now we call the scheduler to inject the wrapped widget
    get_current_scheduler().request_sync_job(|job_builder| {
        widget_binding.set(Some(child), job_builder);
    });

    join_handle.join().unwrap();
}
