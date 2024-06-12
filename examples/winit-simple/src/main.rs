use dpi::LogicalSize;
use epgi_2d::{BoxConstraints, Color};
use epgi_common::{ColorBox, ConstrainedBox, GestureDetector, PhantomBox};
use epgi_core::{SuspendableBuilder, Suspense};
use epgi_material::{CircularProgressIndicator, MaterialApp};
use epgi_winit::{AppLauncher, Window};
use futures::FutureExt;

fn main() {
    let fallback = CircularProgressIndicator!(color = Color::GREEN);

    let app = Suspense!(
        fallback = fallback,
        child = SuspendableBuilder!(
            builder = move |ctx| {
                let (transited, set_transited) = ctx.use_state(false);
                let _res = ctx.use_future(
                    |transited| {
                        tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                            println!("Future resolved with {}!", transited)
                        })
                        .map(Result::unwrap)
                    },
                    transited,
                )?;
                let (pending, start_transition) = ctx.use_transition();
                println!("transited {}, pending {}", transited, pending);

                if pending {
                    return Ok(CircularProgressIndicator!());
                }
                Ok(GestureDetector!(
                    on_tap = move |job_builder| {
                        println!("Tapped!");
                        start_transition.start(
                            |job_builder| {
                                set_transited.set(!transited, job_builder);
                            },
                            job_builder,
                        );
                    },
                    child = ConstrainedBox!(
                        constraints = BoxConstraints::new_tight(
                            if pending { 50.0 } else { 100.0 },
                            if transited { 100.0 } else { 50.0 }
                        ),
                        child = ColorBox! {
                            color = Color::rgb(0.0, 1.0, 0.0),
                            child = PhantomBox!()
                        }
                    )
                ))
            }
        ),
    );

    let app = MaterialApp!(child = app);

    let window_size = LogicalSize::new(400.0, 400.0);
    let window_attributes = Window::default_attributes()
        .with_title("Hello World!")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    AppLauncher::builder()
        .app(app)
        .tokio_handle(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .thread_name("tokio pool")
                .enable_time()
                .build()
                .unwrap()
                .handle()
                .clone(),
        )
        .sync_threadpool_builder(
            rayon::ThreadPoolBuilder::new()
                .num_threads(1)
                .thread_name(|index| format!("epgi sync pool {}", index)),
        )
        .async_threadpool_builder(
            rayon::ThreadPoolBuilder::new()
                .num_threads(1)
                .thread_name(|index| format!("epgi async pool {}", index)),
        )
        .window(window_attributes)
        .build()
        .run();
}
