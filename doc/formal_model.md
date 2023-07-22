

For all async tasks that performed a modification to shared states.
1. Reversible tasks
    1. Normal async work
    2. continue_work_in_subtree
2. Irreversible tasks (No cooperative checking needed)
    1. Anything done while holding the scheduler lock
        1. Interrupt
        2. Commit
        3. Lane marking
        4. Top-level root execution
    2. Sync work



2. Minimal connected regions (MCR) of a batch under a top-level root.





# Concepts:
## Atomicity Guarantees
The following invariants upholds anytime when the global scheduler lock is unlocked

1. All working results from an async batch are either committed or not committed. (Atomicity of async jobs)
2. All sync work are either not executed, or fully-committed, or left suspended. (Atomicity of sync jobs)
3. If a lane is backqueued in a node, then it is not executing in the node's descendants (does not and will not occupy nor be backqueued in any descendants) (Atomicity of cancellation)
4. Consumer-provider subscription coherence for mainline subscriptions and non-mainline subscriptions.
    1. Mainline subscription coherence
    2. Async non-mainline subscription coherence. An async non-mainline subscriber in a mainline node must correspond to a occupied non-mainline node that contains a subscription to that mainline node, vice versa.
    3. (There is no such thing as a sync non-mainline subscription, suspended or not)
5. ~~(Not decided)~~ Lane marking consistency ~~(Eventual consistency or strict consistency?)~~ At any time the lane marking in the entire tree must be consistent with regard to marked lane roots (Since we would need to reuse the lane, thus we will not be able to efficiently doing cooperative cancellation)
    1. Atomicity of lane marking
    2. (Optional) Atomicity of root unmarking
        1. Question: Do we really need to unmark?
    3. Each root (primary/secondary) will have itself and all of its ancestors marked with the corresponding lanes (Sufficiency)
    4. (Optional) Each marked subtree contains at least one root (Necessity)
        1. Question: Does the gain in efficiency worth the burden of lane unmarking?
    5. Each node marked with a lane will have all of its ancestors marked with the corresponding lanes (Continuity)
        1. Continuity would have been implied if both sufficiency conditions and necessity conditions are true. Here it serves as a weakened condition.
    
6. Mainline tree structure consistency. For every mainline node, its parent node acknowledge it as one of its children.


2. Changeset of a batch
3. Minimal connected regions (MCR) of a batch under a top-level root.


# Definition of cancel
1. 