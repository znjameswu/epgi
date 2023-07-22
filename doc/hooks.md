
useDeferredValue

parent
```rust
let now = consumer::<Time>();
let target, set_target = use_state(Default::default());
let current, set_current = use_state(Default::default());
let start_transition, is_transitioning = use_transition();

let new_target = if target == current {now} else {target}
use_effect(|_| start_transition(set_target(new_target)), [new_target])
use_effect(|_| set_current(target), [target])
let child = use_memo(|target| Child(target), [target])
```
child



useSampledValue, but no performance use. (Can be solve by not occupying node when skiprebuild)
```rust
let now = consumer::<Time>();
let target, set_target = use_state(Default::default());
let current, set_current = use_state(Default::default());
let start_transition, is_transitioning = use_transition();

let new_target = if is_transitioning  {target} else if target == current {now} else {target}
use_effect(|_| start_transition(set_target(new_target)), [new_target])
use_effect(|_| set_current(target), [target])
let child = use_memo(|target| Child(target), [target])
```