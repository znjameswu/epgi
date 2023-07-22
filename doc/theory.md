# UI as a Function of States
This section would discuss the high-level, general design of any declarative UI framework. The terms used here are not restricted to this framework only. As a result, we try to avoid describe framework-specific concept here.

## States
A "state" refers to a parameter that can affect the end result view in some way. There are three basic types of states in a declarative UI framework
1. Explicit states: These are the explicit parameters passed in by the developer as the "declarative" description of the UI. Conceptually speaking, for every frame, we describe the entire view as a new tree of explicit states and feed them into the framework. They correspond to the widgets in this framework.
2. Retained states: These are the states that are managed and memorized by the framework in the runtime. They often keep the remaining states and effects from previous computations. They correspond to all the native hooks except provider/consumer in this framework.
3. Contextual states (Optional): These states represent the information the widget can read from its ancestors (i.e. from its position in the tree). They represents the top-down information flow within the UI hierarchy and the positional dependence of a piece of UI. They correspond to the provider/consumer hooks in this framework. In theory, they are optional and their purpose can be achieved by explicit states at the cost of bad API ergonomics and loss of widget abstraction.

They all have their complementary concepts: Implicit states = retained states + contextual states. Ephemeral states = explicit states + contextual states. Local states = explicit states + retained states.

## UI Building Process
The UI building process was never the over-simplified version described in React beginner's documents (`View=f(States)`). 

More accurately, UI should be a hierarchical *tree* structure. The states and UI functions all spread among tree *nodes*. To generate the entire view requires a tree *visit*. The actual UI building process is a recurring visiting process of
$$(S_{\text{This, Explicit}}, S_{\text{This, Retained}}, S_{\text{This, Contextual}})  \overset{\text{UI}_\text{This}}{\longrightarrow} (\text{View}_\text{This}, S_{\text{Child, Explicit}}, S_{\text{Child, Contextual}}, \text{Effect}_\text{This})$$

After unrolling the recursion process, on the very top level, the overall process takes on a more familiar shape
$$(S_{\text{This, Explicit}}, S_{\text{All, Retained}}, S_{\text{This, Contextual}})  \overset{\text{UI}_\text{All}}{\longrightarrow} (\text{View}_\text{All}, \text{Effect}_\text{All})$$

The UI function must be a pure function. That is, if the states are the same, the generated view is guaranteed to be the same regardless of how many times it was called.

The "effect" is abstraction that allows the UI function to be pure. Some operations are unpure in nature, such as network requests and lifecycle hooks. They have to be recorded and managed by the framework to be launched in appropriate timing, or to be ignored if the build results are discarded.

## Interactivity and Incremental Bio;d
Interactivity conceptually symbolizes the generation of a new frame based on an old frame with indeterministic input from the outside world. Since conceptually the only states that are kept between frames are retained states, all interactions will be represented by modifications to the hooks.

Here the interaction is a generalized concept including inputs from humans, from network connections, from OS timers, from resolved Rust futures, etc.

Incremental build is achieved by caching all ephemeral states (explicit states + retained states) within the element node. After an interaction event, we would start the visit from all nodes with a modified hooks and rebuild. If the cached ephemeral states are the same as the new one and there are no modifications to the hooks, we can skip this node and the entire subtree.
## Build Phase Parallelism
There are two types of exploitable parallelism during the building phase: Intra-batch parallelism and inter-batch parallelism.

Intra-batch parallelism is based on the fact that the execution of UI function is independent of each other as long as the ephemeral states are available. When we visit the UI tree to perform rebuild, the tree visit itself can be parallelized. We can make use of Rust's multi-threaded async executors to speed up the visit.

Inter-batch parallelism is based on the fact that a interaction can be solely determined by its modifications to the hooks. If the change sets from two interactions do not overlap and are independent in terms of tree position (that is, no affected node from one change set is ancestor/descendant of another affected node from the other change set), then we can execute two tree visits in parallel, and also commit them independently. Sometimes the conditions can be relaxed and more intra-batch parallelism can be exploited.
## Suspense
Since the UI function is pure and the effects are managed, we can suspend and rebuild a node at any time. This allows the implementation of the Suspense feature. The Suspense feature is a bottom-up error propagation mechanism within the UI tree. If a node contains a retained state that is not ready by the time of the build, it could throw a Suspended error to the nearest Suspense boundaries (which could show a fallback UI piece and hide the suspended subtree). Since we already cache the ephemeral states inside the node, we can safely stash the node and wait. Once the execution can continue, we can come back and use the stashed states to build the node.
## Transition
Transition is an API system designed to directly expose the inter-batch parallelsim capability of the framework to the developers. It allows the developers to declare some specific state changes are not urgent and can be committed at a later time, allowing them to be placed on a different batch and potentially executed in parallel to the urgent ones and other non-urgent ones.


# UI Coherency
## Jobs are Atomic (No Broken Transient Frame)
Some interactions would cause multiple state changes throughout the tree. If we apply inter-batch parallelism among these changes, we could have some inconsistent transient frames (Tearing) before a valid frame with all changes applied appears. On lower end devices or with some bad luck with the async schedulers, such broken transient frames could become a cognitive hazard and even break the logic invariants.

We can introduce a event source tracking on every changes and bundle all the changes from a single interaction event into a "job". A job should be atomic: its changes are either fully committed or not committed at all. A job should be the basic scheduling unit and never allowed to be partially commited.

See also: https://github.com/reactwg/react-18/discussions/70
## Jobs are Total-Ordered if Entangled (No Racing Inputs)
If the change sets from two jobs are not overlapped and also independent (See inter-batch parallelism section), they should be able to be commited in arbitrary order. Moreover, as long as they don't modify the same piece of state, conceptually, they can appear to have been executed in arbitrary order even if they would block each other due to synchronization problems. This approach would allow to exploit more intra-batch parallelism.

However, for jobs modifying the same piece of state (Entangled), we should impose a total order on those jobs. This is a stricter version of the job atomicness: in addition to the atomicness stated before, a job must appears to be commited at some instant of time (linearization point) when observed from another entangled job. 

If we don't, then for two jobs modifying the same set of states, if the modifications happen in random order, a lot of invariants can be broken in the result.

This guarantee would protect the program invariants against multi-threaded racing issues. The choice of the order should follow the spawning frame number of the job and source event order. Implementation-wise, this can be achieve by ensuring a consistent order of state change application while executing a batch.

## Jobs are Progressive (No Stalled Changes)
For jobs that are not entangled but do visit the same node in the tree, although conceptually they are allowed to be executed in any order, practically, they can only execute when the other is not executing due to synchronization problems. Eventually we would need a interruption mechanism to interrupt low-priority jobs for high-priority jobs. This would brings the risk of a livelock: a job gets so constantly disrupted that it may never completes.

This is a major hazard for a parallel scheduler. One way to avoid it is to assign a deadline for each job. Overdued jobs would be prioritized by the scheduler and immune to interrupts unless from another overdued jobs.

# Parallelism And Efficiency
## Tree Visit is a Parallel Process
Aside from the inter-batch parallelism mentioned previously, most other tree visiting process in this framework can be parallelized as well. This include:
### Build phase tree visiting
Described in inter-batch and intra-batch parallelism section.
### Layout phase tree visiting
Similar to build phase parallelism, layout phase parallelism also comes from intra-root parallelism and inter-root parallelism.

Layout will always start from several relayout boundaries (roots). If the relayout boundaries can be split into multiple independent subtrees, then those subtrees can be laid out in parallel. This is inter-root parallelism.

For intra-root parallelism, unlike the build phase, the layout phase exposes way less control for the framework scheduler. Each node would express their layout logic in sequential, turing-complete code, sometimes even with inter-child interactions, which causes problem for parallelization. However, for most multi-child widget, even if their layout logic contains inter-child interactions, they usually layout children in batches. The framework can provide parallel primitives and the widget authors should explicitly use them for parallelism.
### Paint phase tree visiting
Paint-phase parallelism also comes from intra-root parallelism and inter-root parallelism.

## Pipeline Parallelism
Some pipeline phases can be executed in parallel inherently. Thus the execution for the next frame can be started much earlier. For details, please see TODO
## Batching for Efficiency
## Work-Efficient Scheduling
## Incremental Transformation
### Incremental Build
Bypass and reuse the old build results if the states are the same. Explained in the build section. Same as Flutter.
### Incremental Layout
Bypass and reuse the old layout results if the layout constraints are the same. Same as Flutter.
### Incremental Paint
Flutter does not cache the old painting results. However, the painting results is actually cacheable, if we cache the whole paint results of the entire layer in a shared vector buffer. The painting results of a specific node can be represented by the reference to the old buffer and the start/end indices. If the layout constraints are the same, we can directly copy the slices of old painting buffer and bypass the subtree.

# How to Single Pass Layout