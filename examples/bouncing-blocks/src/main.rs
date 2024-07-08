use dpi::LogicalSize;
use epgi_2d::{BoxConstraints, Color};
use epgi_common::{
    AnimationControllerConf, AnimationFrame, BuildContextUseAnimationControllerExt, ColoredBox,
    ConstrainedBox, GestureDetector, Positioned, Stack, ARC_PHANTOM_BOX,
};
use epgi_core::{foundation::Asc, Builder, Consumer};
use epgi_material::MaterialApp;
use epgi_winit::{AppLauncher, Window};
use lazy_static::lazy_static;

use rand::Rng;
use std::{f32::consts::PI, time::Duration};

const CACHE_CHILD: bool = true;

const N_THREADS: usize = 16;

const NUM_BLOCKS: usize = 40000;
const WIDTH: f32 = 1200.0;
const HEIGHT: f32 = 800.0;
const V_REF: f32 = if WIDTH < HEIGHT { WIDTH } else { HEIGHT } / 5.0;
const R_MAX: f32 = 10.0;
const R_MIN: f32 = 5.0;
const DURATION_SECONDS: u64 = 60;

struct BlockDatum {
    color: Color,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    r: f32,
}

lazy_static! {
    static ref BLOCK_DATA: Vec<BlockDatum> = {
        let mut rng = rand::thread_rng();

        (0..NUM_BLOCKS)
            .map(|_| {
                let opacity = rng.gen_range(0.5..=1.0);
                let color = Color::rgba8(0x21, 0x96, 0xF3, (opacity * 255.0) as u8);
                let x = rng.gen_range(0.0..WIDTH);
                let y = rng.gen_range(0.0..HEIGHT);
                let v = rng.gen_range(0.0..V_REF);
                let theta = rng.gen_range(0.0..2.0 * PI);
                let vx = v * theta.cos();
                let vy = v * theta.sin();
                let r = rng.gen_range(R_MIN..=R_MAX);

                BlockDatum {
                    color,
                    x,
                    y,
                    vx,
                    vy,
                    r,
                }
            })
            .collect()
    };
}

fn main() {
    let app = ConstrainedBox!(
        constraints = BoxConstraints::new_tight(WIDTH, HEIGHT),
        child = Consumer!(
            builder = |ctx, animation_frame: Asc<AnimationFrame>| {
                let (x, _animation_controller) = ctx.use_animation_controller_repeating_with(
                    false,
                    AnimationControllerConf!(duration = Duration::from_secs(DURATION_SECONDS)),
                    Some(&animation_frame),
                );
                let time = x * DURATION_SECONDS as f32;
                Stack!(
                    children = BLOCK_DATA
                        .iter()
                        .map(|block_datum| {
                            let effective_width = WIDTH - block_datum.r;
                            let effective_height = HEIGHT - block_datum.r;
                            let mut l =
                                (block_datum.x + time * block_datum.vx) % (2.0 * effective_width);
                            let mut t =
                                (block_datum.y + time * block_datum.vy) % (2.0 * effective_height);

                            if l > effective_width {
                                l = 2.0 * effective_width - l;
                            }

                            if t > effective_height {
                                t = 2.0 * effective_height - t;
                            }
                            Positioned!(
                                l,
                                t,
                                child = Builder!(
                                    builder = |ctx| {
                                        let (clicked, set_clicked) = ctx.use_state(false);
                                        let child = ctx.use_memo(
                                            |clicked| {
                                                ConstrainedBox!(
                                                    constraints = BoxConstraints::new_tight(
                                                        block_datum.r,
                                                        block_datum.r
                                                    ),
                                                    child = GestureDetector!(
                                                        on_tap = move |job_builder| {
                                                            set_clicked.set(!clicked, job_builder);
                                                        },
                                                        child = ColoredBox!(
                                                            color = if clicked {
                                                                Color::rgba8(
                                                                    0xFF,
                                                                    0x98,
                                                                    0x00,
                                                                    block_datum.color.a,
                                                                )
                                                            } else {
                                                                block_datum.color
                                                            },
                                                            child = ARC_PHANTOM_BOX.clone(),
                                                        )
                                                    )
                                                )
                                            },
                                            clicked,
                                        );
                                        child
                                    }
                                )
                            )
                        })
                        .collect()
                )
            }
        )
    );

    let app = MaterialApp!(child = app);

    let window_size = LogicalSize::new(WIDTH, HEIGHT);
    let window_attributes = Window::default_attributes()
        .with_title("Hello World!")
        .with_resizable(true)
        .with_min_inner_size(window_size);

    AppLauncher::builder()
        .app(app)
        .tokio_handle(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(N_THREADS)
                .thread_name("tokio pool")
                .enable_time()
                .build()
                .unwrap()
                .handle()
                .clone(),
        )
        .sync_threadpool_builder(
            rayon::ThreadPoolBuilder::new()
                .num_threads(N_THREADS)
                .thread_name(|index| format!("epgi sync pool {}", index)),
        )
        .async_threadpool_builder(
            rayon::ThreadPoolBuilder::new()
                .num_threads(N_THREADS)
                .thread_name(|index| format!("epgi async pool {}", index)),
        )
        .window(window_attributes)
        .build()
        .run();
}
