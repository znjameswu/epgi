# Datastructure for element

## Notable problems
1. ProviderMap by element nodes or by value? (Decision: by nodes)
    1. By nodes. Don't need special handling when the value changes. Requires extra layer of indirection and potentially a lock.
    2. By value. Horrendous commit changes for every descendants that keeps a provider map when the value changes.
1. Which widgets stores a provider map? (Decision: NUncertain betwen 1/2):
    1. Every widget. 
        1. Horrendous cost to update all provider maps on a tree surgery. But a tree surgery is rare.
            1. Is it even feasible in a multi-threaded framework? Considering the lock cost.
        2. Less layer of indirection
    2. Every widget that introduces a provider (every provider scope).
        1. Low cost to update provider maps on a tree surgery.
            1. But we would still need to visit all descendant no matter what. The visit cost may hide the update cost.
            2. The lock cost
    3. No widget. Build all provider information when visiting from the root. Stupid idea.

1. Consumer subscription

## Green-Red Tree 
ElementNode/ElementInner + Mailbox vs 

Where to store provided value?
1. Element Inner
2. Mailbox
    1. How to commit?


1. Consumer subscription
    1. Add subscription to mailbox
    2. Read provided value from element inner


## Part that needs concurrent access
### The state stored in an ElementNode can be broken down to:
1. Hooks
2. Updates
3. Children
4. Widget
5. Provide
5. ProviderMap
6. Consumers
7. Providers
7. RenderObject (if RenderObjectElement)
8. Fallback (if Suspense)


### Participants that needs to access the element nodes are:

|                   | Hook | Updates | Update Organization      | Parent | Children | Widget | Provide | ProviderMap | ProviderScope | Consumers | RenderObject | Fallback |
|-------------------|------|---------|--------------------------|--------|----------|--------|---------|-------------|---------------|-----------|--------------|----------|
| Event Delivery    | (R)  | W       | No Preference            |        |          |        |         |             |               |           |              |          |
| Lane Markup       |      | R       | Strongly Prefer by Event | R      |          |        |         |             |               | (R)       |              |          |
| Build             | R    | R       | Mixed Preferrence        |        | R        | R      | R++     | R+          | R             |           | [R]          | [R]      |
| Commit            | W    | (R), W  | (Prefer By Event)        | (W)    | W        | W      | W       | W           |               |           | [W]          | [R]      |
| Commit-Deactivate |      |         |                          | W      |          |        |         |             |               | (W*++)    |              |          |

## Decisions


## ArcSwap vs Mutex
### Necessary synchronization points
#### Provider + Pending Consumer synchronization. 
Since reading an ancestor provider is an out-of-tree behavior, thus typical abort-sub-tree-by-lane-marking strategy does not work here. We need to specially handle this case. Registering consumers MUST happen before reading providers. When modifying or commiting the provider value, all pending consumers must be aborted.

Considering the provider modification can be aborted before commit, we should abort and hold any other jobs containing the pending consumer (Rather than allowing other jobs to read modified-but-not-commited values).


    1. This requires a lock synchronization. Lock-free alternative may be sufficient, but may abort more jobs than necessary.
        1. Where to put the lock
            1. Single lock on provider + consumer
            2. Coarse lock on element and lock-free provider & consumer. Worst contention.
    2. Lock-free may actually be necessary. Since any pending modification to the provider value can be rolled-back. 




# Providers vs Normal Hooks
Normal hooks represent "explicit states", meaning they are explicitly passed and owned be the corresponding ElementNodes. Therefore, during building phase, accessing by reference becomes efficient since the reconciler would require exclusive access to the node anyway.

Providers represent "implicit states". They are implicitly passed down by context. As a result, accesses to them are naturally shared and concurrent. Acquiring a reference during build phase will be inherently hugely inefficient due to syncrhonization requirements. Thus only smart pointer accesses (Arc) are available.



# Scheduler generics interface problem
Statement: Scheduler needs to reference an async executor which is currently implemented as a generic parameter. However, we do not wish to expose this generic parameter to the user level code.

1. Trait object over executor seems overly fine grained
2. Trait object over scheduler is a waste of code.
3. Break scheduler up and only expose non-generic part. Achieve runtime polymorphism via message passing.
4. Compile time typedef. Which rips the possibility of a custom runtime.

Where will the non-scheduler code need to interact with the scheduler?
1. Hook invocation.
    1. To report event marking.
    2. To report future completion. (No interaction)
        1. return AsyncOneshotSpsc. No! Multiple batch could be blocked on the same future (An async batch init the hook. A sync batch takes over. Then another async batch entered after sync completed). 
        2. return EventListener.
        2. manually callback to scheduler. No. Multiple batch could be waiting.

# Cooperative yield or forced cancellation.
Possible solutions:
1. Enforced joining of child tasks with top level async cancel.
    1. It has no use! Async cancel won't wait for child tasks, even if they are being awaited!!!!!





# General synchronization problems within batch execution
Problem: We need sychronization between the following operations
1. Batch execution
2. Requested partial abort
3. Batch cancellation
4. Batch completion

## A naive attempt without mutex
### Timeline of a batch execution on a single node
1. Probe availability
    1. Occupy the node: Vacant -> Busy
    2. Request abort if already occupied by another batch
2. Read states (Retained states, implicit states and children widgets)
3. Build and Reconcile
4. Release the node: Busy -> Stashed
5. Write results into the alternate state
### Timeline of a requested task abort
1. Change the state of the node to Busy(requester)
    1. Vacant
    2. Busy(occupier)
        1. Call the target abort handle
    3. Stashed(occupier)
        1. Call the target abort handle
        2. Clear alternate state (RACING!!!!!!)
            1. Can we skip the clear step and just let the requester to overwrite? (NO! The occupier can write at an arbitrarily late point of time under a malicious scheduler!)
        3. Handle subtree


## How to do it with a mutex?
### Timeline of a batch execution
1. Probe availability
    1. Occupy the node: Vacant -> Busy
    2. Request abort if already occupied by another batch
2. Read states (Retained states, implicit states and children widgets)
3. Build and Reconcile
4. Acquire mutex to the alternate state
5. Release the node: Busy -> Stashed
    1. If failed: do not write results.
6. Write results into the alternate state
7. Release mutex

### Timeline of a requested batch abort
1. Acquire mutex to the alternate state
2. Change the state of the node to Busy(requester)
3. Clear any results inside alternate state
    1. If Busy or Stashed, call abort handle and go into the subtree and swap out whatever the target batch has written.
4. Release mutex

### Timeline of a batch cancellation
1. 


## How to do it without a mutex?
### Timeline of a batch execution
1. ArcSwap::compare_and_swap to swap a placeholder data into the alternate state
    1. Request abort if swap failed
2. Read states
3. Build and Reconcile
4. ArcSwap::compare_and_swap to swap the real results into the alternate state, and the placeholder out.
    1. If fail, do nothing.
### Timeline of a requested batch abort
1. ArcSwap::compare_and_swap to swap out whatever inside and swap in a placeholder for the requester.
    1. If Busy or Stashed, call abort handle and go into the subtree and swap out whatevwer the target batch has written.

## Decision
Go with mutex approach for now.



# Suspended Rebuild and State Update
Suspended rebuild needs extra care while handling. More specifically, when they are suspended, they should still commit the state update anyway (or otherwise those update becomes orphans in their job, and later retry won't bother to look for or even be able to find those updates.), ~~but they should not commit the widget update~~, and they have no inner element nor subscirption nor providers to commit. Thus becomes a strange partial commit. Which leaves the "state" part of ElementState up-to-date, but the "effect" part of ElementState stale.

Actually they should commit the widget update, since any async work here can safely skip rebuilding and leave it to this suspended sync work. And we also avoid an inconsistency between "the widget used for the reconcile decision" and "the widget used for the actual rebuild".


# The de-synchronization between the hooks and the element
Hooks must always be up to date (by "up to date", we mean it has merged all the changes from all committed jobs, partially commited due to suspense or fully commited)

Element may be out of date. They do not reflect changes from suspended sync jobs.

The impact: ~~Async jobs cannot skip rebuilding on suspended mainline nodes. Instead, they must try to contend and cancel the suspended work. The suspended mainline nodes may be inflating new nodes that would be affected by the async job. Skipping them may mean some staled new nodes once the suspense completed.~~
(The above analysis is incorrect. Build results is a function of hooks + widgets + providers. )

Ensure any write into ~~the~~ (this exact) RebuildSuspended node (without considering its descendant) will always first cancel the suspended work.


# Consistency problem with subscription
How do async inflating (i.e. non-mainline newly inflated) consumers cancel their subscriptions when they are cancelled?

The main problem is, we can't reliably visit deep into the newly-inflated tree and discover who have they subscribed when we cancel the work.
~~The async build function HAS to store the resulted Element AFTER their child nodes are spawned. OS may decided to interrupt us a year before we try to store the BuildResults.~~
~~Therefore we can never reliably visit an async inflating subtree, unless we synchronized each time when we inflate a child node.~~

~~Update: We can use precommit_effects~~

Update: Now we register children before start any reconcile work. (after rayon migration)

Will eventual consistency and a relaxed invariant work?
1. ~~Problem with a relaxed invariant: A provider update may notify into a living Arc subscriber (upgrade success), and begins lane-marking upwards.~~
2. Therefore, notification to non-mainline nodes MUST not have any effects escape the subtree.



# Problems with mainline secondary root under a to-be-unmounted subtree for an async batch.
Problem: A secondary root may be spawned under a to-be-unmounted subtree. Should we execute this secondary root or not?

If we do, then during cancel, we have to filter through mainline children instead of just the spawned children.

If we do not, how do we avoid from executing it when yielded back to the executor? 
1. Solution 1: Only execute primary roots. (Which is very natural)
2. Solution 2: Do not visit to-be-unmounted subtree at all so we execute nothing. 
    1. It is hard to prevent another tree walk from walking into this mainline subtree
    2. Decision: Rejected

Desicion: 
1. We do not execute the secondary root from the same lane that unmounted this subtree.
2. We will visit the to-be-unmounted subtree and execute qualified work unit.
3. We will not prevent another tree walk from entering the subtree.
4. We will clean up the executing work when we perform the unmount.


# Render object detach behavior on element suspend?
Concensus: If element node A suspends, then any render objects on the the minimal path between A and its nearest ancestor suspense B must be detached.

Question: 
1. Should we also detach render objects of the descendants of A?
2. Should we also detach render objects in the sibling subtress below B?
3. (The above two question should share the same answer)

1. If we do not detach unrelated render objects, then 
    1. (Solved problem) when A resumes, it (and all the render objects between A and B) has to gather child render objects.
    2. When A resumes, A must correctly report itself as a new render object to trigger ancestor updating, even though A's children will report no new render object.
2. If we do detach unrelated render objects, then
    1. This is so hard to impl. FATAL

# Problems with detached renderobject being relayout boundary/repaint boundary.
If we only detach a minimal path from the suspended to the nearest suspense, then there will exist some render subtree without a attached parent render objects. Normally this won't be of much problem since we only need the parent reference for up-marking, which is already covered by element context nodes. However, if we need an ancestor for relayout/repaint boundary, this is problematic.

Question: Will it be OK to have a detached render object as boundary?

Answer: 
1. Seems to be OK for relayout boundaries. Since relayout requires level-ordering, and we have to visit from the top root anyway. The up-marking from a detached relayout boundary will simply be ignored during the visit since they are unreachable.
2. Repaint does not require level-ordering. We could use an AtomicBool flag to check if a repaint boundary is detached and chose to ignore it.


Question: Will it be OK to have a attached boundary with some detached render object in the path?

Inefficient but ok.


Question: How to register boundaries during the first inflating?
1. Relayout boundaries has to be registered during layout. We have to live with this fact and implement null checking. Besides, the null checking is trivial. Because any new render object signal will propagate up by the commit mechanism and get absorbed by the nearest attached ancestor render object. That ancestor is supposed to call its relayout boundary.
2. Repaint boundaries could be registerd statically with an element context node in theory. If we wish to register with a render object, then we have register it dynamically (meaning protected by a mutex). 
    1. If we don't have [UniqueArc](https://github.com/rust-lang/rust/pull/111849), wish to utilize the commit pass for render object creation, and do not want deadlocks and unsafe, then we have to construct render objects from bottom to top. Then a following pass to register repaint boundaries, which would be the layout pass, no-brainer.
    2. If we do have UniqueArc, then we still have to live with a cumbersome two-stage render object creation, which we probably do not want.
    3. If we do not wish to utilize commit pass for render object creation, ......... I can't think of a simple solution.

Conclusion: We do not register boundaries during the build phase or the commit pass. We register it during the layout pass.

Question: How to update detached boundaries to the newest object once the element are unsuspended?

Answer: when a subtree is unsuspended, it is Suspense's job to call its relayout boundary. Then the layout pass will register new stuff according to the conclusion of the previous question. (Problem!!!! Need to make sure no bypass is made in the relevant subtree!)

How to make sure no bypass is made?


# Scheduler Design

The synchronization that a scheduler provides can be catergorized into the following:
1. Non-mainline tree synchronization
    1. Async contention resolution
2. Mainline tree synchronization (R,W)
    1. Mainline tree modifications (W) must also hold non-mainline synchronizations
        1. Sync build
        2. Async commit
    2. Mainline tree read (R)
        1. Layout
        2. Render
        3. Event dispatch

Or a different catergorization method
1. Tree modifications:
    1. Sync build
    2. Async commit
    3. Async contention resolution
2. Tree reads:
    1. Event dispatch
    2. Layout
    3. Render

One major problem: sync build could also happen during layout phase. Layout phase would conditionally need to modify the tree structure.

Under an async executor, it would be easy to impl. However, when using rayon, it seems impossible to yield from a layout work unit into a synchronization scheduler and then yield back the control (unless we handroll CPS transform at the call sites). The synchronization is blocking in nature, and the structured task prevent us from migrating the blocking to a new thread. (https://github.com/rayon-rs/rayon/issues/988#issuecomment-1311961701) 

Decision: Treat the entire layout phase as tree modification. This would waste a lot of potential parallelism for async tasks during the layout phase. But we have no choice as long as we chose rayon + mixed build and layout.


# Committing a suspended async build into mainline
Before the commit, the wake (and the subsequent poll) operation should be executed as fast as possible.

After the commit, they should be batched up with the sync batch.

This creates the question of how to transfer this ownership.

1. Abort the async poll, and forcifully poll for one time to re-register a different waker during commit.
    1. Strang and spaghetti commit logic.
2. Abort the async poll, and unconditionally create a sync poll event for the next frame.
    1. Waste CPU
3. Abort the async poll, and use a flag to indicate whether the previous async poll has been *completed* after the wake. If so, do not create poll event in the next frame. If not, poll in the next frame.



# Problem surrounding the behavior of async SkipRebuild
Skip rebuild can be implemented as not locking the ElementNode in question.

However, if you know in prior that a work will produce a SkipRebuild result, then that work CANNOT be backqueued. Otherwise, on reorder_async_work, it is possible that a SkipRebuild work is chosen which does nothing and returns and then the rest of the work is forgotten forever.
1. Problem: Can you really know in prior that a work will produce a SkipRebuild result. A previously committed work could change the widget so that the next work becomes a SkipRebuild
    1. Answer: Yes you can! We just need to prove that inside the backqueue, there can be at most one Update work and the rest must be Refresh work. So if a previously commit work changed the widget, then the next work must be a Refresh work. And whether a Refresh work will SkipRebuild can be independently determined.