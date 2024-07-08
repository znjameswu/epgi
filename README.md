Warning: Experimental project
---

# EPGI: Exotically Parallel Graphical Interface

`epgi` is a Rust library for building user interfaces.

`epgi`'s design is heavily influenced by existing frameworks like React and Flutter.
- Declarative. Compose declarative widgets and the runtime will efficiently reconcile and update the tree.
- Component-based. Widgets are encapsulated with their own states. They can be re-used and composed into complex UIs.
- **Hooks**. All states and effects are managed by Hooks. (Similar to React Hook)
- **Concurrency**. The framework performs updates in a concurrent manner to avoid blocking. It provides both the **Suspense** APIs for IO-bound scenarios and the **Transition** APIs for CPU-bound scenarios.

Besides, the capabilities from the Rust language and its ecosystem enables us to move further.
- **Parallel**. `epgi` uses the rayon library to achieve maximum parallelism during every phases inside the pipeline, including building, layout, and painting.
- **High performance**. Hot paths are carefully written to minimize heap allocation, runtime polymorphism and locks.
- **GPU-accelerated**. The default 2D rendering backend, vello, is a Rust library with WebGPU-based 2D rendering.
- **Static type safety**. Widgets are bound by its protocol type. Mismatching two unrelated widget results in compile-time error.
- **Extensibility and composability** beyond 2D. The layout protocol can be easily extended to, and composite with, 3D scenes and TUIs.
- (Almost) **Unsafe-free**. The project witnessed a single-digit trivially-provable unsafe usages. We leave most of the optimizations in the hands of compilers for Safe Rust.

# Project status
## "Rocket science" demo
This demo showcases Suspense and Transition API. As well as explicit animation and implicit animation.


https://github.com/ZhennanWu/epgi/assets/38578020/f69d1be5-77cc-4927-8c14-3c827ed6d9e3


## "Bouncing box" demo
This demo tests parallel performance characteristics when rendering 40k interactive & animated widgets, and compare against Flutter

Test result\*:
| N_thread | build+layout (1min avg) | build+layout (low) | paint** | raster | FPS (1min avg)*** | FPS (high) |
|----------|-------------------------|--------------------|---------|--------|-------------------|------------|
|        1 |                 15.6 ms |            14.2 ms |  7.7 ms | 0.7 ms |              29.8 |       39.2 |
|        2 |                 12.0 ms |            10.5 ms |  6.9 ms | 0.6 ms |              36.1 |       49.8 |
|        4 |                 11.7 ms |             9.1 ms |  6.6 ms | 0.7 ms |              37.6 |       53.3 |
|        8 |                 15.1 ms |             9.2 ms |  6.8 ms | 0.7 ms |              32.3 |       51.4 |
|       16 |                 17.4 ms |            10.9 ms |  7.0 ms | 0.7 ms |              29.7 |       49.7 |

\*: Tested on 8-core Intel i7-12700K, Gentoo. Built in release mode.

\*\*: Currently paint phase is not parallelized

\*\*\*: Flutter desktop results: 6 FPS (same implementation, profile mode) / 16 FPS (Flutter optimized implementation, profile mode)

## Objectives
Completed:
- A minimal example is running
- Sync parallel reconcilliation
- Provider and consumer
- Basic hook system
    - `use_state`, `use_memo`, `use_effect`, `use_reducer`
- Suspense and transition
- Async parallel reconcilliation
- Basic hit-test system
- Basic text widgets
- Basic layout widgets
- Prototype animation support

Planned:
- Documentation
- Tests
- Pointer interaction
- Keyboard
- Widget libraries
    - Material design

# Organization of Repo
- `epgi-core`: Core functionalities to spin up `epgi`'s scheduler and pipeline. Does not assume canvas type, backend, or embeddings.
- `epgi-2d`: Basic definitions and utilies for 2D affine canvas rendering. Default to use `vello` as backend.
- `epgi-winit`: Integrations to run `epgi` on `winit`.
- `epgi-common`: Basic widget library to enabling building a minimal 2D UI.
- `epgi-material`: Material design widget library for `epgi`.
- [ ] `epgi-3d`: Basic definitions and utilies for 3D rendering. Default to use `bevy` as backend.
- [ ] `epgi`: Re-export for common library users' convenience.
- ~~`epgi-glazier`:  Integrations to run `epgi` on `glazier`.~~ (Abandoned following Xilem's decision)

# License
Licensed under either of
- Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.
