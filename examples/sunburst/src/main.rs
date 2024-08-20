mod ring;
use std::{f32::consts::TAU, time::Duration};

use dpi::LogicalSize;
use epgi_2d::Color;
use epgi_common::{
    BuildContextImplicitAnimationExt, Center, FlexFit, ImplicitlyAnimated, ARC_FAST_OUT_SLOW_IN,
};
use epgi_core::{scheduler::get_current_scheduler, Builder};
use epgi_material::{MaterialApp, Scaffold};
use epgi_winit::{AppLauncher, Window};
pub use ring::*;

fn main() {
    let sunburst = BoxRingAdapter!(
        child = RingAlign!(
            alignment = RingAlignment::CENTER_END,
            child = Builder!(
                builder = move |ctx| {
                    let (target, set_target) = ctx.use_state((0.0, 0.0));
                    let value = ctx.use_implicitly_animated_value(
                        &target,
                        Duration::from_secs_f32(2.0),
                        Some(&ARC_FAST_OUT_SLOW_IN),
                    );
                    ctx.use_effect(
                        move |_| {
                            get_current_scheduler().create_sync_job(|job_builder| {
                                set_target.set((100.0, TAU), job_builder);
                            })
                        },
                        (),
                    );
                    ImplicitlyAnimated!(
                        value,
                        builder = |_ctx, (inner, span)| {
                            PaddedRing!(
                                padding = RingEdgeInsets::new().inner(inner),
                                child = ConstrainedRing!(
                                    constraints = RingConstraints::new_tight(30.0, span),
                                    child = RingTrack!(
                                        cross_axis_alignment = CrossAxisAlignment::Stretch,
                                        children = vec![
                                            Flexible {
                                                flex: 2,
                                                fit: FlexFit::Tight,
                                                child: ColoredRing!(
                                                    color = Color::RED,
                                                    child = ARC_PHANTOM_RING.clone(),
                                                )
                                            },
                                            Flexible {
                                                flex: 2,
                                                fit: FlexFit::Tight,
                                                child: ColoredRing!(
                                                    color = Color::GREEN,
                                                    child = ARC_PHANTOM_RING.clone(),
                                                )
                                            },
                                            Flexible {
                                                flex: 1,
                                                fit: FlexFit::Tight,
                                                child: ColoredRing!(
                                                    color = Color::BLUE,
                                                    child = ARC_PHANTOM_RING.clone(),
                                                )
                                            }
                                        ]
                                    )
                                )
                            )
                        }
                    )
                }
            )
        )
    );
    // let sunburst = BoxRingAdapter!(
    //     child = PaddedRing!(
    //         padding = RingEdgeInsets::new().inner(100.0),
    //         child = ConstrainedRing!(
    //             constraints = RingConstraints::new_tight_dr(30.0),
    //             child = RingTrack!(
    //                 cross_axis_alignment = CrossAxisAlignment::Stretch,
    //                 children = vec![
    //                     Flexible {
    //                         flex: 2,
    //                         fit: FlexFit::Tight,
    //                         child: ColoredRing!(
    //                             color = Color::RED,
    //                             child = ARC_PHANTOM_RING.clone(),
    //                         )
    //                     },
    //                     Flexible {
    //                         flex: 2,
    //                         fit: FlexFit::Tight,
    //                         child: ColoredRing!(
    //                             color = Color::GREEN,
    //                             child = ARC_PHANTOM_RING.clone(),
    //                         )
    //                     },
    //                     Flexible {
    //                         flex: 1,
    //                         fit: FlexFit::Tight,
    //                         child: ColoredRing!(
    //                             color = Color::BLUE,
    //                             child = ARC_PHANTOM_RING.clone(),
    //                         )
    //                     }
    //                 ]
    //             )
    //         )
    //     )
    // );

    let app = Scaffold!(body = Center!(child = sunburst));

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
