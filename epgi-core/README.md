# Code Organization

- `foundation`: Common functionalities for the crate.
- `sync`: "Sync" pipeline operations that must be executed without disturbances from "async" commits (by holding the global BuildScheduler lock). They are encapsulated under this module to prevent accidental invocation from `async` operations. They include: sync batch reconciliation, batch commit, layout, paint, composite.
- `async`: "Async" reconciliation that can run in background. They are encapsulated under this module to prevent accidental invocation while holding the global BuildScheduler lock.
- `tree`: Common tree definitions and utilities shared by `sync` and `async`
- `scheduler`: Global scheduler responsible for orchestrating the pipeline and event delivery.
- `nodes`: Several core node types that are essential for any application.
- `hooks`: Hook definitions.