use std::{collections::HashMap, time::Duration};

use dpi::LogicalSize;
use epgi_2d::{ArcBoxWidget, BoxProvider, Color, FontWeight, LocalTextStyle};
use epgi_common::{
    Alignment, AnimatedContainer, Center, Column, Container, EdgeInsets, FlexFit, Flexible,
    GestureDetector, Row, Text,
};
use epgi_core::{
    foundation::Asc,
    hooks::SetState,
    nodes::{Consumer2, SuspendableConsumer},
    Builder, SuspendableBuilder, Suspense,
};
use epgi_material::{CircularProgressIndicator, MaterialApp, Scaffold};
use epgi_winit::{AppLauncher, Window};
use futures::FutureExt;
use lazy_static::lazy_static;

#[derive(PartialEq, Clone, Debug)]
pub struct MyAppState {
    requested_hints_count: usize,
}

impl MyAppState {
    fn one_more_hint(&self) -> Self {
        Self {
            requested_hints_count: self.requested_hints_count + 1,
        }
    }
}

struct Hint {
    speaker_name: &'static str,
    content: &'static str,
}

fn main() {
    let fallback = CircularProgressIndicator!(color = Color::GREEN);

    let app_bar = Container!(
        height = 100.0,
        color = Color::rgba8(0x21, 0x96, 0xF3, 0xFF,),
        child = Row!(
            children = vec![
                Flexible {
                    flex: 1,
                    fit: FlexFit::Tight,
                    child: Container!(
                        padding = EdgeInsets::new_all(10.0),
                        alignment = Alignment::CENTER_LEFT,
                        child = Text!(
                            text = "How to build a rocket",
                            style = LocalTextStyle {
                                color: Some(Color::WHITE),
                                font_size: Some(32.0),
                                font_weight: Some(FontWeight::BOLD),
                                ..Default::default()
                            }
                        )
                    )
                },
                Consumer2!(
                    builder = |ctx,
                               state: Asc<MyAppState>,
                               set_state: Asc<SetState<MyAppState>>| {
                        let (pending, start_transition) = ctx.use_transition();
                        Container!(
                            height = 80.0,
                            width = 100.0,
                            alignment = Alignment::CENTER,
                            child = if pending {
                                Center!(child = CircularProgressIndicator!(color = Color::WHITE))
                            } else {
                                GestureDetector!(
                                    on_tap = move |job_builder| {
                                        start_transition.start(
                                            |job_builder| {
                                                set_state.set(state.one_more_hint(), job_builder);
                                            },
                                            job_builder,
                                        );
                                    },
                                    child = Text!(
                                        text = "Request one more hints",
                                        style = LocalTextStyle {
                                            color: Some(Color::WHITE),
                                            ..Default::default()
                                        }
                                    )
                                )
                            }
                        )
                    },
                )
                .into()
            ]
        )
    );

    fn build_hint(id: usize) -> ArcBoxWidget {
        Builder!(
            builder = move |ctx| {
                let (selected, set_selected) = ctx.use_state(false);
                let set_selected = set_selected.clone();
                GestureDetector!(
                    on_tap = move |job_builder| {
                        set_selected.set(!selected, job_builder);
                    },
                    child = AnimatedContainer!(
                        duration = Duration::from_secs_f32(0.5),
                        height = if selected { 70.0 } else { 60.0 },
                        color = if selected {
                            Color::LIGHT_BLUE
                        } else {
                            Color::LIGHT_GRAY
                        },
                        margin = if selected {
                            EdgeInsets::new_all(1.0)
                        } else {
                            EdgeInsets::new_all(3.0)
                        },
                        child = Suspense!(
                            fallback = Center!(child = CircularProgressIndicator!()),
                            child = Row!(
                                children = vec![
                                    Container!(
                                        width = 100.0,
                                        color = Color::WHITE,
                                        alignment = Alignment::CENTER_LEFT,
                                        padding = EdgeInsets::new_all(5.0),
                                        margin = EdgeInsets::new_all(3.0),
                                        child = SuspendableBuilder!(
                                            builder = move |ctx| {
                                                let speaker_name = ctx.use_future(
                                                    |id| {
                                                        tokio::spawn(async move {
                                                            tokio::time::sleep(
                                                                std::time::Duration::from_secs(2),
                                                            )
                                                            .await;
                                                            HINTS_DATABASE
                                                                .get(&id)
                                                                .map(|hint| hint.speaker_name)
                                                        })
                                                        .map(Result::unwrap)
                                                    },
                                                    id,
                                                )?;
                                                if let Some(speaker_name) = speaker_name {
                                                    Ok(Text!(
                                                        text = format!("{} says:", speaker_name)
                                                    ))
                                                } else {
                                                    Ok(Text!(text = "Error: speaker not found"))
                                                }
                                            }
                                        )
                                    )
                                    .into(),
                                    Flexible {
                                        flex: 1,
                                        fit: FlexFit::Tight,
                                        child: Center!(
                                            child = Suspense!(
                                                fallback =
                                                    Center!(child = CircularProgressIndicator!()),
                                                child = SuspendableBuilder!(
                                                    builder = move |ctx| {
                                                        let hint_content = ctx.use_future(
                                                                |id| {
                                                                    tokio::spawn(async move {
                                                                        tokio::time::sleep(
                                                                            std::time::Duration::from_secs(4),
                                                                        )
                                                                        .await;
                                                                        HINTS_DATABASE
                                                                            .get(&id)
                                                                            .map(|hint| hint.content)
                                                                    })
                                                                    .map(Result::unwrap)
                                                                },
                                                                id,
                                                            )?;
                                                        if let Some(hint_content) = hint_content {
                                                            Ok(Text!(text = hint_content))
                                                        } else {
                                                            Ok(Text!(
                                                                text = "Error: hint not found"
                                                            ))
                                                        }
                                                    }
                                                )
                                            )
                                        )
                                    }
                                ]
                            )
                        )
                    )
                )
            }
        )
    }

    let body = Container!(
        width = 600.0,
        child = SuspendableConsumer!(
            builder = move |ctx, state: Asc<MyAppState>| {
                let ids = ctx.use_future(
                    |requested_hints_count| {
                        tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                            (0..std::cmp::min(HINTS_DATABASE.len(), requested_hints_count))
                                .collect::<Vec<_>>()
                        })
                        .map(Result::unwrap)
                    },
                    state.requested_hints_count,
                )?;
                let child =
                    Column!(children = ids.into_iter().map(|id| build_hint(id).into()).collect());
                Ok(child)
            }
        )
    );

    let app = Scaffold!(
        body = Center!(
            child = Suspense!(
                fallback = fallback,
                child = Builder!(
                    builder = move |ctx| {
                        let (state, set_state) = ctx.use_state(MyAppState {
                            requested_hints_count: 2,
                        });
                        BoxProvider::value_inner(
                            state,
                            BoxProvider::value_inner(
                                set_state,
                                Column!(
                                    children = vec![
                                        app_bar.clone().into(),
                                        Flexible {
                                            flex: 1,
                                            fit: FlexFit::Tight,
                                            child: body.clone()
                                        }
                                    ]
                                ),
                            ),
                        )
                    }
                )
            )
        )
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

lazy_static! {
    static ref HINTS_DATABASE: HashMap<usize, Hint> = [
        (
            0,
            Hint {
                speaker_name: "Alice",
                content: "Pointy side up",
            },
        ),
        (
            1,
            Hint {
                speaker_name: "Bob",
                content: "Flamey side down",
            },
        ),
        (
            2,
            Hint {
                speaker_name: "Carol",
                content: "Get up",
            },
        ),
        (
            3,
            Hint {
                speaker_name: "David",
                content: "Get down",
            },
        ),
        (
            4,
            Hint {
                speaker_name: "Eve",
                content: "Do not explode (optional)",
            },
        ),
        (
            5,
            Hint {
                speaker_name: "Jebediah Kerman",
                content: "*Kerbal squeaks*",
            },
        ),
    ]
    .into_iter()
    .collect();
}
