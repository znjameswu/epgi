use std::sync::Arc;

use epgi_2d::{BoxConstraints, Color};
use epgi_common::{ColorBox, ConstrainedBox, GestureDetector, PhantomBox};
use epgi_winit::AppLauncher;

fn main() {
    AppLauncher::new(Arc::new(GestureDetector {
        on_tap: Some(Arc::new(|| println!("Tapped!"))),
        child: Arc::new(ConstrainedBox {
            constraints: BoxConstraints::new_tight(50.0, 50.0),
            child: Arc::new(ColorBox {
                color: Color::rgb(1.0, 0.0, 0.0),
                child: Arc::new(PhantomBox {}),
            }),
        }),
    }))
    .run()
}
