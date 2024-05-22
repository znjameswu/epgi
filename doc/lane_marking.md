
Decision: 
1. Allow relaxed lane marking. Do not propagate lane unmarking.
2. Use reverse-order unsubscription to deactivate consumer root.

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
~~Relaxed lane marking with no unmarking~~

Please see schedule algorithm document


# Parallel lane marking algorithm that support subtree-scoped dynamic node deletion
## Motivation
Some unit of work will spawn secondary root unit of work deep down their subtree (usually caused by a provider update). To work-efficiently visit and rebuild, we need to also lane marking those secondary root unit of work. When we need to abort a unit of work, then the spawned secondary roots should be destroyed, and the lane marking must be updated to reflect this change to stay consistent.

## Caveat
1. Sometimes two units of work will spawn secondary roots at the same node. Therefore the secondary root should not be destroyed if only one spawner unit of work is aborted.

## Candidate Solutions
0. Do not support partial abort at all. Always abort an entire batch.
0. Do not use bit mask for lane marking. Use a children lane counter for lane marking.
1. Breadth-first bottom-up unmarking. (With spawner reference count)

    Very bad parallelism and high serialization. Work-efficient.
2. Bubble-up unmarking after SeqCst children lane query. (With spawner reference count)

    > For a pair of atomic operations on M called A and B, where A writes and B reads M's value, if there are two `memory_order_seq_cst` `std::atomic_thread_fences` X and Y, and if A is *sequenced-before* X, Y is *sequenced-before* B, and X appears before Y in the Single Total Order, then B observes either:
    >
    > - the effect of A
    > - some unrelated modification of M that appears after A in M's modification order

    and

    > There is a single total order S on all `memory_order_seq_cst` operations, including fences


    Not work-efficient. When a node with N children total and M lane-marked children, the children query cost if O(MN) in the worst case.

3. Parallel collect all remaining roots in the subtree, clear all lane marking in subtree and then remark (With spawner reference count)

    Not work efficient. Any abort would have to visit all the way down an arbitrarily deep subtree, since a higher unit of work could have spawned a lot of extremely deep secondary roots and we would need to collect them all, clear them all and remark them all.
4. Abort all the way up to highest root. (Use separate lane marker for secondary roots) (No spawner reference count required)

    Work-efficient for its purpose. Not very work-efficient compared to rest of the solutions, since it also needs to visit and clear an entire subtree.

5. Use an auxilliary, scheduler-exclusive lane marking to mark to-be-unmarked roots. Use this auxilliary marking to achieve node bypassing during a top-down recursive mark computation. (With spawner reference count)

    Can work inside an inverted Roslyn Red-Green tree!

The reference count can be replaced by storing the spawned root in the **first** spawner that actually spawns it.

Decision: Auxilliary mark with ref-counting.

## Alternative to the lane unmarking algorithm
Non-intrusive node design gives us the ability to fold on children's lane marks during the return phase of a visit. That usually happens during commit or cancellation (what?).
 
In short, it gives us the ability to sum over children's lane mark most of the time.

Detailed explanation:
1. Why do we need lane unmark in the first place?
    - Other work may eventually result in loss of consumers subscribing to the current provider, either by changing widget, or unmounting some ancestor element to that consumer. (What about the other work writes the same value into the current provider and thus nullifies this provider update?)
    - Loss of consumers may result in loss of consumer roots, which may results in smaller and fewer work regions, which is important for subtree bypassing (consider a scenario where other work constantly spawns and remove new consumers). 
    - Therefore we need some sort of consistency-keeping mechanism to reflect changes caused by loss of consumers
    - On the other hand, adding new consumers is of no concern. We can just mark again and it is self-healing.
    - If the other work doesn't change consumer nodes at all, then it is even less of a concern.
    - "Unmark-and-later-remark" is just one of the easiest mechanism to reason about. It completely annihilates any side effect of a work and later recompute the side effects of that work from scratch, no matter if the interfering work is adding or removing consumers.
2. We can make the interrupter lane responsible for restoring lane marking consistency of the interrupted lane (with ref-counting)
2. The interrupter could only interrupt/occupy the writer lane because
    1. Either widget update, or event update, or subscription update

Impact:
1. This means allowing inconsistent (excessive) lane marking state to exist. We only guarantee consistency of *a given lane* in the *currently occupied* or *occupiable* work regions *of that lane*.
2. What if the interrupting work itself gets interrupted again?
    1. The new interrupter, if we task it with unmarking the original lane, may skip and not visiting some consumers needs to be unmarked
    2. ~~Therefore, we implicitly mandate that the interrupter work, under any circumstance, must execute before the interrupted, which means a total order between batches. ~~ (This is nonsense. All problems comes because the interrupter work being interrupter. If the interrupter yields then there is no need for lane unmarking to start with )

### Self + subtree marking vs Self + descendant marking?
Recurrence relation:

Self + subtree marking: Parent subtree = Parent self + \sum child subtree

Self + descendant marking: Parent descendant = \sum (child self + child descendant)

At the unmarked secondary root:

Self + subtree marking: New self = mailbox + poll. New subtree = self + \sum child subtree (Requires to visit children, while the children itself may be pending an update)

Self + descendant marking: New self = mailbox + poll. No need to update descendant.

Therefore, we choose self + descendant marking to unlock strict unlocking capabilities.




Or we can also do this by a deletion with SeqCst fence children inquiry at each level



All the the above requires a global scheduler lock to avoid exposing the transient states.

# Fundamental problems with any kinds of lane unmarking algorithm and its alternatives
Syncrhonization between unmarking and marking proves to be difficult:
1. Interlane unmarking and marking synchronization: solvable using lane mask to prevent unmarking of other lanes that are not part of unmarking target.
2. Intralane unmarking and marking synchronization.
    1. Marking and unmarking are both propagating upwards. They creates racing problems!
        1. In order to decide whether to unmark upwards, unmarking will *always* need to collect lane marks from sibling nodes to see if the sibling has the lane.
        2. Meanwhile, there could be a lane marking coming up from the said sibling nodes.
        3. If the unmarking reads an older version of sibling, and commits later into parent, then the results of lane marking is destroyed! We now have an inconsistent lane marking state.
    2. This racing problem is hard to avoid
        1. Since the starters of the two operations are in different subtree, they cannot be synchronized by holding a lock on a single node. Theoretically, holding a lock on the lowest common ancestor node between the two is sufficient, but until the race actually happens, every ancestor node could be such common ancestor node from the perspective of either side. We can't hold lock on every ancestor. That will trash the lane marking system.
        2. Syncrhonization by batch lifecycle is also difficult:
            1. If we allow partial abort and allow async lane marking, then it will always be possible for one subtree to be unmarking and another subtree to be marking up. No matter you do the unmarking during scheduler reordering or during sync commit. Because the core design objective is to allow the async work to run in parallel to the sync work.
            2. If we disallow partial abort, then we have the guarantee that unmarking and marking up won't be happening at the same time. BUT now there is no point in performing any kind of unmarking, since unmarking serves the sole purpose of partial abort.
            3. What if we only allow the scheduler to perform lane marking (aka sync lane marking)? (*candidate*)
                1. The async work request a lane marking when handling node A, then proceed to A's descendant and yield the subtree B back to scheduler. We need ensure that lane marking A must happen before yielding subtree B. Putting the two types of scheduler work in the same queue should do the trick.
                2. Clarification: lane marking not immediately visible before yield back the subtree is not a problem. Missing consumer lane mark can cause false node skips during async tree walk. But if a false node skip is generated, this means there is only a consumer update in that node, which can be handled by scheduler when processing yielded tree from the exact node skip.
        3. Tweak the unmarking and marking process so that they don't meet in the first place (*candidate*)
            1. The best ordering guarantee we can have under a partial-abort-enabled and async-lane-marking design is that, under the subtree to be aborted, there can be no marking up. (If you go anywhere out of the subtree, marking up could be happening at any time)
            2. Then we must restrict the scope of lane unmarking to the current subtree to be aborted. And this is actually do-able
                1. The scope of lane marking also needs to be changed to allow it to be unmarked by the now much limited unmarking process.
                    1. Consumer lane marking start should start from the consumer, up to the changed provider that triggered this consumer work.
                    2. For a given consumer node and a given lane, only the first lane marking will actually mark up, because the following lane mark will always comes from providers that are lower in tree than the provider that triggers the first lane marking, and thus their path will already be marked.
                2. For a given consumer node and a given lane, when the consumer refcount decrement to zero, the abort that triggered this decrement must come from above the topmost changed provider A (aka the provider that triggers the first lane marking) (if the abort comes from somewhere below, then the refcount from A won't be decremented and thus refcount won't be zero), and thus we have the guarantee we won't meet any lane marking when we unmark the limited scope from the consumer to A.
                3. Then the unmarking itself is not implementable. Suppose along a tree path from top down there is 4 nodes: A (provides T1) -> B (provides T2) -> C (consumes T2) -> D (consumes T1). When we abort at A, we see that B-C pair should not affect us but we have no way to know it since we can't know the existence of B when we visit A, but when we visit B, we can no longer unmark the nodes above B below A.
                    1. Reverse-order unsubscription. We unmark in the exact reverse order of how we would mark. That is, when we lane mark, we start from top down, so when we unmark we start from bottom up (unmark at the upwalk phase).
            3. The design
    3. 

# Alternative to consumer node refcount
Can reverse-order unsubsription solve the need for ref-counting? Yes it can!