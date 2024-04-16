use epgi_2d::{BoxConstraints, Color};
use epgi_common::{ColorBox, ConstrainedBox, GestureDetector, PhantomBox};
use epgi_winit::AppLauncher;

fn main() {
    AppLauncher::new(
        GestureDetector::builder()
            .on_tap(|| println!("Tapped"))
            .child(
                ConstrainedBox::builder()
                    .constraints(BoxConstraints::new_tight(50.0, 50.0))
                    .child(
                        ColorBox::builder()
                            .color(Color::rgb(1.0, 0.0, 0.0))
                            .child(PhantomBox::builder().build())
                            .build(),
                    )
                    .build(),
            )
            .build(),
    )
    .run()
}
