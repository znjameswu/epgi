use epgi_2d::{BoxConstraints, Color};
use epgi_common::{ColorBox, ConstrainedBox, GestureDetector, PhantomBox};
use epgi_core::{SuspendableBuilder, Suspense};
use epgi_winit::AppLauncher;
use futures::FutureExt;

fn main() {
    // let app = GestureDetector::builder()
    //     .on_tap(|| println!("Tapped"))
    //     .child(
    //         ConstrainedBox::builder()
    //             .constraints(BoxConstraints::new_tight(50.0, 50.0))
    //             .child(
    //                 ColorBox::builder()
    //                     .color(Color::rgb(1.0, 0.0, 0.0))
    //                     .child(PhantomBox::builder().build())
    //                     .build(),
    //             )
    //             .build(),
    //     )
    //     .build();

    let app = GestureDetector!(
        on_tap = || println!("Tapped"),
        child = ConstrainedBox!(
            constraints = BoxConstraints::new_tight(50.0, 50.0),
            child = ColorBox! {
                color = Color::rgb(0.0, 1.0, 0.0),
                child = PhantomBox!()
            }
        )
    );

    let fallback = GestureDetector!(
        on_tap = || println!("Fallback tapped"),
        child = ConstrainedBox!(
            constraints = BoxConstraints::new_tight(30.0, 30.0),
            child = ColorBox! {
                color = Color::rgb(1.0, 0.0, 0.0),
                child = PhantomBox!()
            }
        )
    );

    let app = Suspense!(
        fallback = fallback,
        child = SuspendableBuilder!(
            builder = move |mut ctx| {
                let res = ctx.use_future(
                    |_| {
                        tokio::spawn(async {
                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                            println!("Time has passed!")
                        })
                        .map(Result::unwrap)
                    },
                    (),
                )?;
                Ok(app.clone())
            }
        ),
    );
    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("tokio pool")
        .thread_stack_size(3 * 1024 * 1024)
        .enable_time()
        .build()
        .unwrap();
    let tokio_handle = tokio_runtime.handle().clone();
    let rayon_spawn_handler = |thread: rayon::ThreadBuilder| {
        // Adapted from rayon documentation
        let mut b = std::thread::Builder::new();
        if let Some(name) = thread.name() {
            b = b.name(name.to_owned());
        }
        if let Some(stack_size) = thread.stack_size() {
            b = b.stack_size(stack_size);
        }
        let tokio_handle = tokio_handle.clone();
        b.spawn(move || {
            let _guard = tokio_handle.enter();
            thread.run();
        })?;
        Ok(())
    };
    let sync_threadpool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("epgi sync pool {}", index))
        .spawn_handler(rayon_spawn_handler.clone())
        .build()
        .unwrap();
    let async_threadpool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .thread_name(|index| format!("epgi sync pool {}", index))
        .spawn_handler(rayon_spawn_handler)
        .build()
        .unwrap();

    sync_threadpool.broadcast(|_| {
        let _guard = tokio_runtime.enter();
        std::mem::forget(_guard);
    });
    async_threadpool.broadcast(|_| {
        let _guard = tokio_runtime.enter();
        std::mem::forget(_guard);
    });
    AppLauncher::builder()
        .app(app)
        .sync_threadpool_builder(sync_threadpool)
        .async_threadpool_builder(async_threadpool)
        .build()
        .run();
}
