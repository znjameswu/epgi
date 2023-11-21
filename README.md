*Warning: Experimental project
---

# EPGI: Exotically Parallel Graphical Interface

EPGI is a Rust library for building user interfaces.

EPGI's design is heavily influenced by existing frameworks like React and Flutter.
- Declarative. Compose declarative widgets and the runtime will efficiently reconcile and update the tree.
- Component-based. Widgets are encapsulated with their own states. They can be re-used and composed into complex UIs.
- **Hooks**. All states and effects are managed by Hooks. (Similar to React Hook)
- **Concurrency**. The framework performs updates in a concurrent manner to avoid blocking. It provides both the **Suspense** APIs for IO-bound scenarios and the **Transition** APIs for CPU-bound scenarios.

Besides, the capabilities from the Rust language and its ecosystem enables us to move further.
- **Parallel**. EPGI uses the rayon library to achieve maximum parallelism during every phases inside the pipeline, including building, layout, and painting.
- **High performance**. Hot paths are carefully written to minimize heap allocation, runtime polymorphism and locks.
- **GPU-accelerated**. The default 2D rendering backend, vello, is a Rust library with WebGPU-based 2D rendering.
- **Static type safety**. Widgets are bound by its protocol type. Mismatching two unrelated widget results in compile-time error.
- **Extensibility and composability** beyond 2D. The layout protocol can be easily extended to, and composite with, 3D scenes and TUIs.
- (Almost) **Unsafe-free**. The project witnessed a single-digit trivially-provable unsafe usages. We leave most of the optimizations in the hands of compilers for Safe Rust.

# Project status
- [x] A minimal example is running, demonstrating sync reconcile, layout, paint, composite, scheduler, hooks, provider.
- [ ] Reconciliation for async batches
- [ ] Hit-test and event distribution
- [ ] Keyboard
- [ ] Complete set of hooks
- [ ] Widgets library for actually useful UI building

# Organization of Repo
- `epgi-core`: Core functionalities to spin up EPGI's scheduler and pipeline. Does not assume canvas type, backend, or embeddings.
- `epgi-2d`: Basic definitions and utilies for 2D affine canvas rendering. Default to use `vello` as backend.
- `epgi-glazier`: Integrations to run EPGI on `glazier`.
- `epgi-common`: Basic widget library to enabling building a minimal 2D UI.
- [ ] `epgi-material`: Material design widget library for EPGI.
- [ ] `epgi-3d`: Basic definitions and utilies for 3D rendering. Default to use `bevy` as backedn.
- [ ] `epgi`: Re-export for common library users' convenience.

# License
Licensed under either of
- Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.