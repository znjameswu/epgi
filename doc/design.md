The state of each job
1. Created
2. Scheduled
3. Executing
4. Suspended
5. Commited(EOL)



The state has to be managed inside an Arc counter. In order to be able to abort a job and discard the work, we cannot allow user-facing api to consume the old state or the action, or at least hold onto the old state if needed. Therefore, the user cannot reuse the old state anyway and they need to construct new instances everytime. We have several choices after that:
1. Let each dispatched action consume the older state. And every dispatch need to provide cache the older state. On aborting the job, we need to visit all the dispatched action in order and restore state. (Pretty pointless)
2. Manage the state by Arc.  All actions can only access the state by reference. The sync jobs however will need to pay the same cost as async jobs.


The fundamental conflict in element tree design
1. Click event can only be dispatched by RenderObject's hittest. The event will be delivered by RenderObject's binding into the element tree
2. With each async job, element tree will experience constant reference changes.
3. Async job commit phase has to be as short as possible. The binding changes can cost O(N).
4. The event dispatch is independent to the commit system. Changing bindings during commit **may cause loss of event**!

Choices:
1. Binding by node reference
2. Binding with forwarding node reference.
    1. Commit by set a forward reference from the old node to the more recent node, and the old node will be treated as pure tomestone and clear itself of child references.
    2. Pro: RenderObject's binding can self-heal to main branch.
    3. Cons: This is still O(N) commit phase cost. Each self-heal RenderObject require one forwarding reference set during commit phase.
    4. (Dispute) Pro: If child hold a strong reference to parent, then we can reuse child node and expect their parent reference to self-heal.
    5. (Dispute) Pro: If we add extra fields to Rc's header (create a new smart pointer) and use it to store forwarding references, then we can reuse child node and expect their parent reference to self-heal.
3. Binding by mailbox with reference to the main branch node.
    1. Similar to binding with forwarding node reference
4. Binding by mailbox.
    1. Pro: Constant commit phase cost.
    2. Cons: For fast job marking, each mailbox has to hold reference to its parent mailbox.
    3. Cons: For up-to-date context within a job's subtree, the element tree has to maintain a parent node referencing mechanism, which duplicates with the above.
    4. Cons: Mailbox does not hold reference to the main branch. But starting a job needs main branch node. The scheduler has to start searching from the main branch root. Introduce a O(M lgN) searching cost at the start of each job. (M is number of node with updates within a job)
        a. Pro: each frame would only need one search.
        b. Pro: searching can find conflicts more quickly and consistently. More optimizations for the scheduler.
    5. Cons: Is there anything more that the RenderObject need to know about the main branch?
    6. ~~Cons: It makes job contention resolution MUCH harder. Suspended jobs need to hold reference to the newest version of node, which, if not stored inside mailbox, has to be book-kept by the scheduler since contending jobs will update the element node.~~ (Not necessarily the newest version of node. An older version of the parent can also work.)
    7. Cons: It makes job contention resolution MUCH harder. To perform a subtree abort, the scheduler cannot easily locate the corresponding work node, unless visit all existing work nodes inside the job. (which *might* not be expensive)

# Why not use an inverted Roslyn Red-Green Tree and bypass the mounting phase?
Let's call the downward pointing tree as the local tree and the upward pointing tree as the context tree.

The advantage of this structure is avoiding the mounting phase at all. Node inflating follows a strict construction order of "ancester context node -> descendant context node -> descendant local node -> ancester local node" order with no transient state.

The problems with this structure are:
1. Node order. The local tree is constructed bottom-up. Needs to be extra careful when firing effects.
2. Unable to partial abort (interrupt). Interrupts require a top-down tree walk which can only be achieved on the local tree. However, the local tree is constructed at the end of the build phase.
    Answer: inflating cannot be interrupted anyway. Interrupt on mainline rebuilds and unmounting work well.

    Question: Non-mainline rebuilds and other uncommitted subscriptions remain a problem. Subscriptions must be registered as weak local node references.

    Solution: 
    1. We can introduce a epoch system for providers. An epoch records the last commit that has modified the given provider. For build task without an available local node reference, they should compare the provider epoch tag when they finish their build, and trigger non-mainline rebuilds if the epoch changes. (Potential problem: local nodes are constructed bottom-up. The non-mainline rebuilds needs level-ordering. Solution: can be done by provider tracking.)
    2. Record all uncommited subscriptions as weak context node references. (Wrong! In this solution we will not be able to interrupt mainline uncommited subscription even during commit time)
    3. Record non-mainline subscriptions as weak context node references.and only converts them to weak local node references during commit time. (Similar to solution 1)
    4. Record non-mainline subscriptions as atomic bool flag.

    Question: Rebuilds with children inflating will have divergent control flow. Rebuilds are non-blocking but inflatings are blocking.

    Solution: Keep all the control flow as blocking. Give reconciler to the task rather than yielding task to the reconciler.

        Further problems: If we give the control flow to the reconciler, then if we allow the async build function, then we will have no way to tell if an execution is suspended whether to the async executor or to the build logic. We would have to use fallible build function, which is proven to be less elegant during a resumed inflate.

        This issue is the direct consequence of a diverted control flow. The ancestor work's lifetime (referring from the start to writing to async output / sync commit) is not deterministic relative to the children work. 

        Or to say that the continuation of a suspended work contains other polymorphic async logic. That is the problem.

        Solution: Nested future? async fn xxx（） -> impl Future. We require the user implemented Element to write like this. Sure like a hell to look at though.
        ```rust
        let result: FutureOr<_> = widget.build();
        let result = result.join().await;
        return async move {

        }
        ```

        Future problem 2: Suspend in descendant becomes unworkable.

        (**UNRESOLVED DEFECT**)


3. Partial unmarking (Solved). Normal bottom-up partial unmarking requires to know about sibling information. Sibling visiting requires a first-up-then-down visit capability. However, the Green-Red tree can only achieve first-down-then-up visit capability.

## Secondary problems with bypassing mounting phase.
Bypassing mounting phase would require a spawn_scoped semantic for all child tasks, because the ancestor has no way of accessing the AbortHandle of the inflating child which does not even have its node yet. Using spawn_scoped is not ideal for `async-std`

However, this may not be too much a problem, since spawn_abortable_scoped is zero cost.
### Is spawn_detached/spawn_relaxed usable?
A non-blocking abort itself is theoretically impossible to synchronize with the effect reversal, it must visit the critical section of the affected shared state somehow. So we can effortlessly call abort handles during these visit. Therefore, as long as we can visit all the affected nodes, then we can use spawn_detached.

Use spawn_relaxed should also be possible, though we don't know implementation-defined execution path is good.


Async job execution choices:
1. Async jobs construct whole new subtree copies and commit by swapping subtree roots.
    1. Pros: Fast commit.
    2. Cons: Slow build time.
    3. Cons: More job conflictions.
2. Async jobs construct new node copies on demand and commit by swapping the roots of connected parts of the new subtree, as well as updating the parent pointers of all preserved children under rebuilt nodes.
    1. Pro: Reduce job confliction and increase parallism.
        1. The increased parallelism comes from false job conflictions that are impossible to detect in the scheduling phase. It is detectable during execution phase tho.
    2. Cons: Slow commit time.
    3. Cons: Too hard to implement.
3. Between 1 and 2, only specializes for top-level `Provider`s. `Provider`s will be specially treated during scheduling. Top-level `Provider`s will be swapped with children preserved. The updates falls to their listeners.
    1. We can't specially treat providers until the batch enters execution. Any special treatments during the markup phase will risk leaving outdated consumer informations.



Job Yielding by stashing or by waiting mechanisms (such as CondVar or tokio::Notify)?

Decision: yield by waiting


## Do we need a parent pointer inside element?
Scenarios needing parent pointers:
1. Accessing inherited widgets. (Can be done by caching inherited widgets at each level)
    However, we need to distinguish normal hook widget with provider to keep the cache under control
2. Rearrange renderobjects. (Render objects can store their own parent)
3. Registering context? (No we don't need parent pointers)

inherited cache vs parent pointer
1. Cons: Everytime a new inherited widget is inserted in the structure, the whole subtree needs to be visited. (Complexity: all descendants minus descendants under descendant inherited widget.)
    With parent pointers, we still need to do this because maintaining cache consistency is inevitable.


Since we have to start working from the root, the cache can be replaced by the internal state of the reconciler?
No cache:
1. Pro: no need to visit to the bottom to replace all caches on structural change. (Structural change always invalidate everything regardless! unless tree surgery)
2. Cons: computation cost
Cache pointing to provided instances:
Cache pointing to provider mailbox and mailbox stores instances:



## Should the provider be a normal hook widget?
`use_provider`
`use_provider_mut`
`use_consumer`


## General problems associated with provider
If we need to register consumers, that means our work can escape the subtree scope, which is bad. Although the registration does not mutate state and won't interfere other async jobs, however, the sync job can mutate states and interfere with newly inflated nodes (The sync job cannot know the existence of the new node)!
1. Register during async build.
    1. Abort descendant async jobs.
    Which might be crazy, imaging a root sync timer.
    2. Async job leaves a `dependant` mark and clears them during commit. See below.
    3. Async job carries on the lane mark in the update queue of new nodes and let sync job abort them.
        1. Need to prevent lane mark pollution because the new node's update queue still has references all the way up.
            It is the async job who disposes the node, but it's the sync jobs lane that needs to be cleared. Racey like hell. We want each job to exclusively handle their own lane mark.
        2. Do we really need to prevent that?
            We need to allow for inconsistent lane mark (from the sync job's perspective)
            Cause unnecessary visits for the sync job.
        3. We are forced to clear lane mark during the commit.
            Not a bad thing. Because async job can be aborted so they better clear lane mark during commit.
        4. The lane mark got a lot more harder to clear.
            Easier way would require dynamic recording of all dynamic updates on consumers. 
            Async lane reclamation?
    4. 
2. Collect dependencies and register during commit.
    1. leave a `dependent` mark.
        Timing problem. We need to ensure async jobs can never be commited during the sync job, and aborted async jobs will be aborted after the sync job.
    2. Reject at commit. (Huge waste of cpu)

# Some guarantees regarding to provider/consumer
1. If two jobs become entangled at a consumer hook, then they must already have been entangled at the provider hook.

## Internal scheduler guarantees
1. 



## Suspense
We have to stash the work

Stashed work is stored as a collection of unfinished terminal work nodes. Each terminal work node carries an up-pointing work node chain. Each work node represents a new element node to be created to replace original element nodes.

The job can be resumed in part by simply working on any of the terminal work nodes.

### How does the stashed sync work and the Suspense interact.
The terminal work nodes are separately handed over to their closest Suspense ancestors.
#### What if there is a Suspense inside the work node chain.
We have to specialize this case. Any work node above this Suspense must immediately be completed and all work node chains that come across here need to be cut.

## Safety of lane marking and job aborting
The lane mark primarily serves two purposes: job conflict heuristics and job aborting. We will show the safety of job aborting.
1. False positives. This means we falsely abort a non-conflicting batch. False positives can only occur when a lane retirement is not synced into the executing tasks. We use a global RwLock to ensure lane retirements are synced with every tasks: by forcing tasks to acquire a reader lock before requesting an abort and lane retirement to acquire a writer lock.
    1. To reduce lane retirement synchronization cost, we accumulate up completed lanes and retire them in a single attempt when the lanes are filled up.
2. False negatives. This means we failed to detect a conflicting batch. If batch A did not report conflicting batch B at node X, this means A has already started executing and B's marking at node X is not synced into A's task. A's marking at node X happens before A's execution. Since access to the mark at node X is totally-ordered, we have A's marking happens-before B's marking. In tasks of batch B, they will observe A's marking happens before their execution, thus making sure B to request A's abort. 
3. The above are about the correctness of requests. An abort request can fail if the requested to-be-aborted batch has just been commited. What should we do then?
    1. Request with a scheduler Mutex
        1. Double loading pattern: Load the node -> report conflicting lanes according to mailbox (withe the mutex)-> Load the node again from parent.
        2. Abort the requester: When we have the mutex, abort the requester instead.
    2. Request without a mutex via atomics/channel. In this case, the scheduler must check the atomics/channel before a commit.
        1. Can we use something similar to double loading pattern here? 

## Design of stashed work
Abort can only be *performed* by the inner scheduler!

## What if an async work touches a stashed suspended work.

## It is very difficult to handle suspensded error in a currently inflating element.


## Protocol as generic parameter or associated type?
This is a stupid question. It MUST be associated type.

# Lifecycle of a job

# Lifecycle of a batch
```
      ┌─────────────┬─────────┐
      │       Update│         │
 ┌────▼────┐  batch │    ┌────▼────────┐
 │Destroyed│        └────┤Batch Created│
 └────▲────┘             └────┬────────┘
      │                       │Lane Assignment
      └─────────────┬───►┌────▼────────┐
              Update│    │Lane Assigned│
              batch ├────┴────┬────────┘
                    │         │Mark-up
                    │    ┌────▼────┐
                    ├────┤Marked-up│
                    │    └────┬────┘
                    │         │Execute
                    │    ┌────▼────┐◄───────────┐
                    └────┤Executing│            │
                         └────┬────┴────────────┤
                              │                 │Abort
                         ┌────▼────┐            │
                         │Completed├────────────┘
                         └────┬────┘
                              │Commit
                         ┌────▼────┐
                         │Committed│
                         └─────────┘
```

# How to store the state of suspended new element?
There are two ways to store them: in work nodes representing pending operations without merging them into the element tree, or introduce a "suspended" state in the element node.
1. Work node tree
    1. Cons: Extremely complex work tree design especially for multichild element. Requires synchronization to avoid racing. Requires a slot mechanism to memoize place of insertion.
    2. Cons: Hard to represent "interrupt now and come back later" workflow.
        1. Possible solution: Merge what could be merged synchronously, and leave the rest as a tree of work node representing pending operations
            1. Cons: The tree of operations will be leaking up into the ancestor in order to insert new children into the ancestors.
            2. Cons: Would be complex to determine which part was suspended and where to come back to. A suspended new element will drag its ancestors into suspense if its ancestors is also newly inflated widget.
    3. Cons: Would be impossible to detect conflicts with new jobs. A new job could very well abort this suspended subtree.
2. Suspended state for the element nodes
    1. Pro: very easy to implement
    2. Pro: easy to detect conflicts with new jobs.
    3. Cons: THIS IS INTRODUCING A NEW UNRENDERABLE STATE. We have to make SURE that the suspense counter is ALWAYS CONSISTENT otherwise we face panics during follow-up phases.
    4. Pro: the work node tree above is actually isomorphic to this approach. To achieve the same optimization, a tree with isomorphic structure and synchronization guarantees is unavoidable.

Decision: Suspended state for the element nodes.


# Problems with strict build-then-layout

We need a LayoutBuilder equivalent widget to extract layout information during the build phase.

## Potential Solutions
1. Flutter-style interleaved build and layout
    1. Complete interleaved build and layout
    2. Flutter style partial interleaved build and layout
2. Caching the last layout information with hooks
    1. Ask to provide a default value on the first build.
        1. Wrong in logic
    2. Return None on the first build. Let user explicitly handle.
        1. Very ugly and non-ergonomic
    2. Allow to use suspense fallback on the first build
        1. Impossible. The fallback will intercept any layout information from above.

## Problem with hooks + suspense
Suppose we somehow solved the problem of fallback intercepting layout information.
Suppose a widget tree of: A -> B -> C

Layout logic of A:
1. Layout B with width constraint [50, 200]
2. Read width of B
3. If width of B larger than 150, relayout B with width constraint [100, 300]

```
B = Suspense!{
    fallback: |err| {Container!{width: 100}}, 
    child: C
}
```

```
C = LayoutBuilder!(|constraints| {
    if constraints.max_width > 250 {
        return Container!{width: 100};
    } else {
        return Container!{width: 160};
    }
})
```

First frame: C suspended. B fallback to 100. A pass down [50, 200]. A read 100. Somehow [50, 200] passed through to C.
Second frame: C returned 160. B cancelled fallback. A pass down [50, 200]. A read 160. A pass down [200, 300].
Third frame: C returned 100. A pass down [50, 200]. A read 100.
Fourth frame: C returned 160. .......

Note: This oscillation is universal to any caching strategies and does not depend on the behavior of Suspense and its fallback. Thus caching is not a viable option.

## Problem with interleaved build and layout
1. If we encounter suspended build during layout then it would be a hell to deal with a new fallback. The closest Suspense may very well be out of the scope of the layout builder, and the relayoutBoundary of the closest Suspense may very well be even more out of scope.
    1. Issues with LayoutBuilder
        1. Requires LayoutBuilder to have a suspense fallback and handle suspended error! 
        2. Requires no suspended error ever reaches LayoutBuilder.
    2. Issues with Slivers (Lazy lists)
        1.


# Other problems



8. Rasterization




## What happens when a frame become overdue (janked)?


## Define exactly what "abort"ing a batch means?
Functionally, there are three categories of aborting:
1. Yield. The batch stops execution but can keeps its work and record how to resume.
    1. This should be implemented cooperatively in the batch execution code.
2. Rollback. The batch stops execution, and let scheduler decide which part of the work can be kept and where to resume. The source of this operation can come from anywhere in the tree.
3. Cancel. The batch stops execution and scheduler totally wipes everything it has done. The source of this operation should only come from the root.

## Should we support partial abort (Rollback)?
Definitely would make the resume code look like hell, since we would be hand-writing a CPS (continuation-passing style) transformation.

(Do we really need CPS in build phase abort?)

1. CPS would be quite possible if we are writing in rust.
2. Difficult if we got more than one abort requests on the same batch. Would be some kind of CPS with very complex state.
3. If we implement CPS, then we must support both async and sync cases.
    1. Async case would be pretty straight-forward to reason about.
        1. What if the abort point is way above the currently processed nodes?
    2. For the sync case, We have to definitively separate the abort error with suspended error.

Actually stashing the explicit states (widget) would be enough

For the work on a node, there will always be two sources of cancellation. One from the the cancellation of the node above, one from external trigger.

If we use a cancel-on-drop model with partial abort support, then there will always be transfer of handle ownerships.



# Use async/await for suspense or use custom hook and error?
Open question. Use custom hook for now



# Provider wrapped by box or by arc?
1. Cheaply clonable type
2. Expensively clonable type

1. Mainline owning rebuild
2. Async clone
    1. Cheaply clonable type
        1. Box
        2. Arc
    2. Expensively clonable type Arc<T>
        1. Box<Arc<T>>
        2. Arc<T>


Advantage of box:



# Use concurrent reclamation instead of Arc?
Motivation: https://pkolaczk.github.io/server-slower-than-a-laptop/

https://rust-lang.github.io/rfcs/2580-ptr-meta.html

When thin pointer become stablized and crossbeam-epoch has proper support. 


# Should we drop or retain the RenderObject when the Element is suspended?
Retain: May reuse the RenderObject before the suspense. Problem: How to handle the suspended RenderObject. How to guarantee liveness of the RenderObject when reattached. Attached Object now have to occupy one field for every element.

Drop: Easy to implement.

# Suspense/Fallback as a hook?
It will be a lot more easier to impl fallback in a static, declarative way rather than a dynamic way.


# Why tokio instead of rayon?

1. Because async jobs need a way to suspend and wait for potentially something like a network request!!!!! 
    1. You basically need to hand-roll an async scheduler bookkeeping logic to properly store the future and wake yourself. I'm very sure your version and mine can't beat tokio in this regard. 
2. Using synchronization primitives inside rayon can easily lead to deadlock footgun. (https://docs.rs/rayon/latest/rayon/fn.join.html#warning-about-blocking-io)
3. Specifically for rayon, its performance is bad for small amount of element. See https://github.com/rayon-rs/rayon/issues/1044. Tokio, on ther other hand, seems to be a very safe bet. If we perform the single-element-optimization, it will be great.
4. We are not strictly CPU-intensive in the conventional sense. Despite that the parallelized building process could be computationally heavy, we still would yield to the scheduler at every node boundary. We can't cause a blocking (in the sense of starving the executor) if we code properly. So what we are doing is just heavy concurrent jobs with a high CPU usage.



# Relaxed or strict lane marking?

## Definition


Relaxed lane marking can only be used as a heuristic.

## Advantage
If we do relaxed lane marking and opt out of lane unmarking, we could even ~~use a plane AtomicLaneMask instead of a locked LinearMap~~ drop the spawned secondary root record entirely!!!!!!(Huge advantange)

## Problem with async secondary root spawning & unmount
If async secondary root spawning and lane marking is performed within the async task itself, then this process has no point of synchronization with a potential unmount that removes the subscription.
1. Due to lock ordering, while unmounting a subscriber, we cannot grab the lock of the provider ElementNode. The reverse is true, while spawning secondary task, we cannot grab the lock of the subscriber ElementNode.
2. The only two possible synchronization points are:
    1. In the secondary root lock:
        1. This could be potentially replaced by a lockfree primitive, so better not relying on this lock.
        2. Lock a secondary root lock then check an cooperative flag seems a strange code pattern.
    2. In the provider lock:
        1. Perform spawning and lane marking while holding the provider lock
            1. This would make an async provider write potentially way too expensive. This could affect the performance of the sync batch since they also need to lock some providers, especially those "common" providers with a lot of subscribers.
        2. Otherwise this provides no synchronization. The thread can suspend and the lane marking can happen a year later.

## Decision
Relaxed lane marking with no unmarking


# Layout tree walk implementation

Layout is different from build, so the impl strategy from build tree walk does not work here:
1. Layout visit process (child layout) is offloaded to user specified logic. The users may request a specific sequential layout order instead of parallel independent layout. The users may also retrieve child layout result (sizes) midway.
2. A single render object could perform many layout attempts during a single layout phase. Such layout attempts will be multiplicative when going deeper into the tree and is unpredictable in nature. From a non-root render object, it has no way of knowing if a speicific layout visit will be the last.
    
Edit: 
1. With lane marking, we CAN go deeper to visit all marked nodes, but we cannot tell if we SHOULD go deeper in the current layout pass (which may only be a probing pass).
2. ~~With lane marking, if we choose to not go deeper beyond necessary, we cannot tell if a node has been relayout-ed in the last layout scope.~~

However, layout still prefers level-ordering.

As a result, it will be difficult and ugly if we use lane marking for layout. Since we cannot know if a layout visit will be the last and cannot perform unmark. (Even if we do unmark, then we need ref counting)

We have following candidate solutions:
1. Flutter style queue. Execute from least depth. Wasted parallelism
2. Up-searching for lane mark to make sure level ordering and only perform layout for top-level relayout boundaries.
    1. Execute in batches. Lose some parallelism
    2. Track blocking relations between relayout boundaries. The blocking relations form an isomorphic tree. When one relayout boundaries completes layout, the relayout boundaries it blocks will be visited and executed if not covered in ancestor layouts.
3. Tree walk a subtree after a relayout boundary completes layout. The layout itself does not attempt tree walk beyond instructed by user.
4. Solution 2-2 and 3 actually have the same execution flow.

Decision: Prototype with Solution 1 and later try Solution 2-2 or 3. Solution 1 is easy to upgrade to Solution 2-2.

# Detached layout optimization (for !parent_use_size) is incorrect

Therefore we can get rid of that ugly LayoutExecutor, since now every layout operation is structured.


# Composite phase design notes

1. Layers with cached encodings are a necessary construct to impl repaint boundary (or incremental painting)
2. LayerScopes act as bindings between layer tree and the render object tree. Important note: LayerScope tree is not isomorphic with render object tree.
    > ## Problem: LayerScope generic over canvas or over protocol?
    > Advantage of generic over canvas:
    > 1. Reuse vello's Fragment append mechanism.
    >
    > Decision: generic over canvas. However, RepaintBoundary can be generic over a subset of protocol whose transform corresponds with canvas.
3. LayerFragments are ephemeral and acts as filler for leftover render objects that are not covered by the LayerScope tree.
4. LayerFragments has to cache encodings to composite with its sibling LayerScopes.
5. Therefore, if LayerScope cache its encodings, the encodings will duplicate with the combined encodings from all its children (LayerScopes and LayerFragments)
    > ## Problem: Should we hold duplicate encodings in LayerScopes
    > Advantage:
    > 1. skips recompositing
    > 
    > Disadvantage:
    > 1. Potential waste of space of up to depth of the layer tree
    > 
    > Decision: No duplicate encodings in LayerScopes. LayerScopes are transparent and do not hold encodings.
6. This makes composition happens at RetainedLayerScope level. Since compositing an unretained LayerScope has no left-over effect.
7. RetainedLayerScopes with cached bitmaps (or other universal render results) are a necessary construct to impl recomposite boundary (or incremental composition)
    > ## Problem: Do we really want incremental composition?
    > Yes we want. Flutter says retained rendering by retaining bitmaps improves performance on certain render effects.
    > 
    > Moreover, I can't think of a way to render 3D scenes or videos in vello other than passing bitmaps. We already need infrastructure to pass around bitmaps anyway.
8. This introduces a RetainedLayerScopes tree structure which has to be bidirectional to propagate recomposition event.
    > AHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH, another bidirectioanl stuff
    > ## Problem: Can we express this bi-directionality in the ElementContextNode?
    > Actually, yes. As long as we strictly binds LayerScope tree to the element tree (i.e. not allowing external references and modifications) and keep it uni-directional.
    




# New Paint and Composition design
## Cache the paint results for each render object or for each layer fragment?
### How to cache for each render object
Each render object can hold a slice into the encodings of its parent layer fragment (By and Arc and index range)
### Problem
Vello can only encode absolute transformation in its encoding.

Then the relative transformation betweeen the render object and its layer fragment (or, in a worse implementation, its root canvas) becomes a problem. Since the slice we are referencing are using absolute transformations, we need to invert the old relative transformation before applying new relative transformation, if it has changed. 
1. However, the relative transformation may be non-invertible or lose precision.
2. We have to store the relative transformation betweeen the render object and its layer fragment for every render object. Which may be very expensive
3. Or we could enforce that each transformation MUST create a new layer, so that there will be no relative transformation for each render object. Which sounds wasteful and could lead to abuse of render objects as layers.

### Deicision
Do not cache encodings for each render object by slice. Only cache for each layer fragment.

## Layout of Layer
As the previous subsection discussed, there are the following states that the layer tree would have to store:
1. Child layers
2. Detached child layers
3. Transformation of each child

### Absolute transformation or relative transformtion
We can either store the absolute transformtion between the layer to the canvas root, or the relative transformation between the layer and its parent layer.

If we store the absolute transformation, then when a layer's transformation gets updated, all its descendants' transformation has to be updated as well. This is not incremental painting (Ancester repaint causes unconditional whole subtree repaint).

If we store the relative transformation, then 
1. When we compositing the layers into the canvas, we have to keep a transformation stack. (Not a big problem)
2. We cannot start compositing on arbitrary layers since we do not have its absolute transformation. We can only start compositing on canvas layers. (Composition has to be done in this way. Working as intended)
3. When we perform hit test, we have to start from canvas layers and walk down. (Working as intended)

Decision: Store relative transformation
### Where to store the transformation
We can either store the relative transformation in the parent or in the child

When only the relative transformation changed, we should not touch children in theory.

Decision: Store in the parent

### Where to store the detached child layers
We can either store the detached child layers 
1. In the parent node they choose when they are painted. When painted, we **walks up** the layer tree and find the parent node it wants.
2. In the immediate parent node and wait for the composition phase. The child will not meet its chosen parent in the painting phase. In the composition phase, when **walking down** the tree we either keep a layer stack or return a detached children stack, only then we attach the detached children to its chosen parent.
    1. We still need to store the detached children in its chosen parent's child list, in order to perform hit test.

Decision: Go for immediate parent node for now.

### Problem: Nested detached layers adoption behavior

A detached layer A under another detached layer B could miss its adopter C due to B chooses an adopter above C.

#### Solution 1: Fully detached and eager resolution during adoption
We do not hide A under B. When resolving detached layers for any layer above B, they will see A and B simultaneously. Therefore, C will successfully adopt A even when B remains unadopted.

Problem: 
0. Eager resolution of detached layers under a subtree
    1. A layer has to know and report all detached childrens in its subtree, even if they are deep descendants down the tree.
1. Dynamic child detection WHILE compositing.
    1. FACT: A layer must know all its adopted child layers and child fragments before compositing to its parent. 
        1. Example: any retained rendering mechanism
        2. Example: 3D to 2D adapter layer. A single adopted object could change the rendering results of the whole scene.
    2. FACT: Most layers are transparent. Generating encoding caches for them is wasteful.
    3. PROBLEM: Stateless API design becomes hard.
        1. Retained rendering demands collecting detached layers before finishing compositing.
        2. Transparent layer demands no extra states to be introduced.
        3. Leaving all of these to user will expose the layer adoption logic, which is not supposed to be user modifiable.
        4. Introduce an associated retained state type `Layer::RetainedState`
2. Detached layer ordering problem
3. Detached subtree has been eagerly resolved, which means we should not resolve detached child layers anymore when they are composited, which means a different behavior compared to structured child layers.


> Generic Function Builder Pattern
> 
> https://stackoverflow.com/questions/37606035/pass-generic-function-as-argument

## How to perform hit test
We can use three types of offset when invoking a hit test method on a render object:
1. Absolute offset relative to the canvas (Not possible since we decided to store relative transformations in layers)
2. Relative offset relative to the parent render object
    1. Translate offset at each render object 
    2. Offset definition problem? Protocol::Offset is defined to position a span, not a pointer position.
        1. We would need a dedicated Protocol::HitPosition for pointer position rather than Canvas::HitPosition. (Unacceptable)
3. Semi-absolute offset relative to the parent layer
    1. Each render object need to store its semi-absolute transformation to its parent layer during the painting pahse.

Decision: Use semi-absolute offset. Optionally introduce Canvas::HitPosition

## Render Object state when performing hittest
Flutter says the hit-tested render objects must be already laid out, but not necessarily painted. Flutter's RenderOpacity employs an optimization where painting is skipped when opacity = 0

The same optimization technique might be problematic when we have detached children. If we skip painting a subtree, then its detached chidlren will also be skipped. For example, for a selectable text with opacity = 0, if we skip painting it, then its text selection overlay and handles, as they are detached, will not be registered to the layer tree, making them also invisible. Which is a bad breaking user experience compared with opacity = 0.001.

Also, we decided to use semi-absolute offset for hit testing. Which requires the render objects to store up-to-date painting results.

Therefore, we cannot both have detached layer children design and the flutter painting optimizations.

Decision: Hit-test can only be performed on painted render objects.



# Two choices in implementing an isomorphic tree subset
Problem statement: ElementNode tree -> RenderObject tree,  RenderObject tree -> Layer tree. These two mappings is called isomorphic tree subset.

1. Container managed. Containee use an associated type to indicate the mapped node type. The mapped node is stored inside the container
    0. Good type safety and zero cost abstraction.
    1. For container node without a mapped node, the associated type in the containee has to be `Never`
    2. Very bad extensibility. 
    3. Can enforce a constant instance binding.
2. Containee managed. The mapped node is directly stored inside the containee.
    1. Extensibility when coupled with runtime type reflection such as the type registry in `bevy-reflect`.
    2. Implementers of containees can easily break lifetime guarantees.
    3. Users of containees may also easily break lifetime guarantees since this field is exposed to them.
    4. Operations on the mapped node may require fields in the container which are not accessible to the containee.
    5. If implemented without an associated type, then the caller can only get a trait object. Possibly not zero cost.

Temporary decision: Keep Element -> Render implemented as container mangaged for now. Try implement Render -> Layer as containee managed to see the results.

# Problem with layer lifecycle
Statement: We have no *efficient* method of detect whether a layer was left unpainted during the paint phase.

Problem 1: This can have an undesired effect of propagating false repaint and recomposition event from an unpainted child layer.

Solution: Record each layers last painted frame id. Stop propagation if found outdated.

Problem 2: Bloated layers not reclaimed until render object goes out of scope

Solution: Leave as-is. To sync two set of tree structures requires some sort of reconciliation mechanism, which is inefficient in nature. We can do it between element node tree and render object tree, but not appropriate for render object tree and layer tree considering the duplicity and the dynamic nature of painting (in contrast with the static nature of isomorphic element-render mapping).

Decision: We do not implement any layer lifecycle mechanism. The lifecycle of a layer is stricly bound to its render object.

