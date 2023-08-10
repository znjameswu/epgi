mod state;
use std::sync::Arc;

use epgi_2d::{BoxConstraints, BoxProtocol, RenderRootView, RootView, RootViewElement};
use epgi_core::{
    common::{create_root_element, ArcChildWidget, Element, ReconcileItem},
    scheduler::{get_current_scheduler, Scheduler, SchedulerHandle, TreeScheduler},
};
pub use state::*;

pub fn run_app(app: ArcChildWidget<BoxProtocol>) {
    // let root: ArcChildWidget<BoxProtocol> = Arc::new(RootView { child: app });
    let widget_placeholder = Arc::new(RootView { child: None });
    let element = RootViewElement { child: None };
    let element_node = create_root_element(widget_placeholder, element, BoxConstraints::default());

    let tree_scheduler = TreeScheduler::new(element_node.clone());
    let sync_threadpool = rayon::ThreadPoolBuilder::new().build().unwrap();
    let async_threadpool = rayon::ThreadPoolBuilder::new().build().unwrap();
    let scheduler_handle = SchedulerHandle::new(sync_threadpool, async_threadpool, todo!());

    let scheduler = Scheduler::new(tree_scheduler);
    let join_handle = std::thread::spawn(move || {
        scheduler.start_event_loop(get_current_scheduler());
    });

    let widget = Arc::new(RootView { child: Some(app) });
    let reconcile_item = ReconcileItem::new_rebuild(element_node, widget);

    join_handle.join().unwrap();
}
