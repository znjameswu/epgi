# Hook design

Hooks encapsulate two fundamental capabilities: state and effect.

Basic characteristics (in an async build context):
1. State 
    1. A state needs to be read from the element node (or init from hook), used in build phase, then written back into the element node during commit.
2. Effect
    1. An effect is produced during build phase, then fired during commit.
    2. When fired (during commit), it produces a teardown.
    3. A teardown also needs to be written back into the element node during commit.
    4. However, a teardown could hold onto non-cloneable resources, and thus cannot be cloned. 
        1. A teardown is also usually susceptible to double-free bugs. Even if we force the users to provide a `Clone` teardown, it is not a good idea to clone it around. Might trigger a double-free somewhere.
    5. Therefore, a teardown should not be read from the element node and then kept during build phase, it should only be touched during sync commit phases.
    6. Therefore, *a teardown is definitely not a state*. 

Therefore, our design should have *two* distinctive types used during build. One that can be read out of ElementNode, and one that can not.

Major difficulty:
1. The commit is pretty overloaded. However, commit overhead is crucial for inter-batch parallelism.
    1. Commit needs to overwrite state, obviously
    2. Commit needs to fire the new effect
    3. Commit needs to decide whether to teardown the old effect
    4. Commit needs to write the new teardown.

## Naive design
Have a `Hook` and `HookState`. Everytime we build, we read a `Hook` from `HookState`. When we commit, we reconcile an updated `Hook` with `HookState`

Problem:
1. The read is polymorphic and comes with new allocation.
2. The commit is polymorphic.
    1. And firing effect/teardown inside reconcile is polymorphism inside polymorphism
3. Even pure state hook needs to go through polymorphic reconcile, when in theory an overwrite should be enough.
4. Each impl-ed `HookState` needs to correctly manage their effect during commit.


## Design
Have a dedicated `Effect` trait for effect and `Hook` trait for states. The ElementNode holds `Vec<(Box<dyn Hook>, Option<Box<dyn Teardown>>)>`. The build context returns `Vec<(Box<dyn Hook>, Option<Box<dyn Effect>>)>`
1. Pure state hook does not return effect, so when they commit, they simply overwrite the hook. No virtual function!
2. Effect hook returns an effect, so when they commit, they overwrite the hook, call the teardown, and fire the effect and overwrite the teardown.
3. The commit always has one less virtual function cost compared to naive solution!
4. Implementers no longer need to manage effect lifecycles