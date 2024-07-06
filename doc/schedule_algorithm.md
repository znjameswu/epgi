# General Idea of Scheduler Dispatch Algorithm

To schedule multiple batches and and their subtree concurrently and preemptively requires synchronization mechanism. There is global mutex of [LaneManager] to provide syncrhonziation. However the global mutex cannot be used on every contending node since the contention would severly limit the concurrency of the program. We need to implement a fast path to synchronize operations on nodes, hence we need to prove the correctness of the scheduling algorithm.

There is a permit on every node. It has the following state:
1. Vacant
2. Modified
3. Subscribed









1. Pending unit of work can only be spawned by either: 
    1. Ancestor provider modifications
    2. Interrupts from a higher priority lane
2. Pending unit of work can only be destroyed when its spawner unit of work is being destroyed







# Concepts
## Mainline, WIP, Non-mainline, Uncommited
Mainline tree refers to the tree reachable by visiting on the `mainline` version of each tree node.

Work-in-Progress tree for a specific lane refers to the tree reachable by visiting on the `mainline` version of the nodes that are not occupied by this lane, and visiting on the `work_in_progress` version of the nodes that are occupied by this lane.

Non-mainline subtrees refer to those subtrees that belong to some WIP trees, but unreachable by mainline tree. Non-mainline subtrees must have a mainline ancestor. A non-mainline node's descendants must all be non-mainline.

Uncommited changes refer to the changes that have been submitted by some lane, but not commited yet, so they are not visible in the mainline tree. However, they can exsit on mainline nodes. Notice the reachability vs visibility. Example: during a rebuild, a node adds a consumer on a value, which means the provider node receives a uncommited, but mainline consumer.
## Level-ordering
**Definition**: This word is used for a lack of better words. Given a tree and multiple nodes we wish to visit, a visit strategy will be called *level-ordering* if: if any two nodes we wish to visit possess ancestor-descendant relation, then completing the visit on the ancestor must be happen before starting the visit the descendant.

**Motivations**: In the building phase, a unit of work is allowed to spawn arbitrary unit of work in its children nodes (and descendant nodes in the provider's case). Therefore, an ancestor root unit of work may spwan units of works all the way down to its descendant, encompassing or even overwriting the descendant root unit of work. If we started executing descendent root before completing the ancestor root, then potentially we are performing wasted work. It is not work-efficient. A level-ordering execution strategy guarantees work efficiency.

**Implications**:
1. It is easier to implement in a serial algorithm.

### Level-ordering for general build phase tree walk
There are three ways to do it
1. Breadth-first top-down tree searches with descendant lane marking as search hint.
2. Node up-querying of ancestor chain information before execution. This requires extra waking mechanism for descendant nodes if the ancestor node visit fails to visit the descendant.
3. Subtree pre-emption by lane marking. Lane-mark and pre-empt descendant from executing. This also requires extra waking mechanism for descendant nodes, and the easiest way is just a inefficient version of the breadth-first top-down tree search.

### Level-ordering for non-mainline subtree
Non-mainline subtrees suffer from another problem. You cannot perform reliable descendant lane marking on those subtrees since they are by definition unreachable from the mainline tree, which makes lane marking useless. (Though a pessimestic tree walk that visit both mainline and non-mainline trees should be able to utilize the lane marking, it would require a better coding guideline on the non-mainline locks to avoid dead locks.) 

Luckily, the only two ways for a tree walk to interact another lane's non-mainline results are 1. contend with the other lane on a mainline ancestor of the non-mainline subtree, and 2. change a provider that is consumed by a still-inflating subtree. The former is well handled by the interrupt system. The only case that poses a challenge is changing the provider without contending any mainline ancestor of the non-mainline subtree.

There are several ways to solve it:
1. Node up-querying before execution.
2. Provider tracking during the non-mainline subtree's build phase. In the top-down build phase, we can track which providers we have subscribed in all the occupied ancestors, thus when a still-inflating node requires a non-mainlined subscirption, we can know whether it is a subscription root. Any descendant node can be discarded as in a 






# Algorithm
1. Interrupt will always destroy the entire (mainline) subtree of the work from the same lane. Formally, there does not exist any executing work nor interrupt stash of the same lane below an interrupt stash of a certain lane (in the mainline tree). If they exist prior to the interrupt, they are destroyed.
2. Interrupt strategy: when a unit of work is executing on a node, it will request to interrupt on the following nodes if there are contending lanes executing on them:
    1. Its children
    2. Its provider subscriber nodes in the mainline tree.
    3. Newly-inflated non-mainline tree nodes with subscription to its provider. Rather than an interrupt, this will cause immediate re-execution on those nodes.

        Problem: It is mostly work-efficient but not entirely. Since the newly-inflated nodes are guaranteed to be only accessible from its inflater lane, we can achieve level-ordering simply by batching up the interrupts since an interrupt will always clear the entire subtree. However, the newly-inflate nodes could be affected if the interrupter later visited their mainline ancestor, or later visited another provider that they also have subscription on. This is marginally work-inefficient but tolerable.

        FLAW: What should we do about newly-susbscribed, mainline tree nodes???

        FLAW: What if the newly-inflated lane commit earlier than the provider update? Inconsistency!!!

        Solution 1: The provide update will interrupt non-mainline subscriptions in paranoia. What if the non-mainline node gets aborted entirely? How to reach these nodes during a commit?

        Solution 2: Spawn a new WorkGuard for the inflater lane held by the interrupter for each new, uncommited root consumer. The WorkGuard will prevent the inflater lane from committing. If the interrupter get commited/interupted at the provider node, or if the uncommited root consumer gets interrupted/destroyed, the corresponding WorkGuard should be dropped.

        1. Provider update is executed before the inflating and has an earlier deadline.
        2. Provider update is executed before the inflating and has a later deadline.
            The inflating ignores the provider update and insert the pending subscriber entry. When the provider update gets commited, it will schedule a non-mainline rebuild if the inflating is still not commited.
        3. Provider update is executed after the inflating and has a later deadline.
            The provider update ignores this inflating root when it is executed. When the provider update gets commited, it will schedule a non-mainline rebuild if the inflating is still not commited.
        4. Provider update is executed after the inflating and has an earlier deadline.
            The provider update ignores this inflating root when it is executed. When the provider update gets commited, it will schedule a non-mainline rebuild if the inflating is still not commited. If the inflating is committed earlier, then it should be committed. 
        5. Problem: The inflating commit would add subscriber to the provider and it would mean dynamic async spawning of secondary root unit of work. Which is very, very difficult to guarantee level-ordering, unless we abort some ancestor nodes.
         FATAL!!!!!! We can't get reference to local node from a context node. Interrupt requires a local node reference. (This operation is essentially work in the lower tree interrupt higher work, cannot be expressed by inverted Roslyn tree)
         FATAL2!!!!! This is unfair to the provider updater, since it can be interrupted by a lower priority batch. Which breaks the progressive guarantee.

         Solution
            1. When a provider update tries to occupy a node, it will also read all the uncommitted consumers. If there are, the fast occupy fails and the scheduler is invoked to arbiter. For uncommited consumers with less priority, the scheduler will demand a CommitBarrier. For uncommited consumers with higher priority, the scheduler will ignore.
            2. Whenever a batch gets commited, all node that was subscribed with uncommited root subscription will be reexecuted. Reexecution = first interrupt then immediately requeue.
            3. We will store a weak pointer to the local node 
        Review:
            1. Provider problem is actually the only case where simultaneous occupation on the same piece of state from multiple lanes is allowed in the algorithm, in contrast with the one occupier only strategy for local states.
            2. Therefore, there are multiple strategies to handle it
                1. Race-to-commit strategy: This strategy actually won't cause deadlock. But will cause livelock.
                2. Strict RwLock semantics based on priority. The suspended sync consumer have be a special case under this strategy and would probably need to use race-to-commit strategy anyway (Or if we regard sync uncommited as commited? (What about suspended rebuild?)).
                    1. ~~Flaw: The subscription can only be determined at the runtime. A re-execution may yield different subscriptions. Thus a re-execution must first clear all previous subscriptions and then re-register every single subscription.~~ (Solved by static subscription)
                    2. There are following impls
                        1. Blocking flavor. Violations to priorities are solve by interrupting the existing ones and blocking the unspawned ones.
                        2. Barrier flavor. Low priority ones gives their commit barrier to the provider and execute as normal, they just can't commit before this barrier is dropped. Whoever gets commited triggers re-execution on the other one.
                            1. Disadvantage. More CPU time. If N consumers are contending the same provider, the provider update will be re-executed by up to N time. (Racing! There are no cooperative flags for a re-execution root (but does not matter?))
                            2. Advantage: Easier to implement, less state. 
                            3. Advantage: Can implement reinflate optimization in case of high priority provider update (actually, reinflate optimization is very hard).
                                1. Reinflate optimization means to reuse some results from the should-be-aborted inflate (because now the state it keeps is stale), without throwing everything out during abortion. Ideally we would perform a rebuild on a previous inflate result.
                                2. Difficulty: you would run into problems such as rebuild on a AsyncInflate node.
                            4. Advantage: Can implement new consumer re-execution optimization in case of high priority new consumer.
                                1. New consumer re-execution optimization optimization means that, when a provider is being updated and new consumer is committed and added, instead of re-execute the entire provider update work, we only re-execute to reflect the provider update on the new consumer.
                            5. ADVANTAGE: NO NEED TO UNBLOCK! WOW!
                                1. If a work gets cancelled, there is no need to unblock potentially blocked 
                        Decision: barrier flavor
            ~~Temporary desicion: Use race-to-commit .~~
            Decision: Strict RwLock semantics with barrier flavor (with optimizations not implemented)
            Rationale: This is a precious parallelism oppurtunity not presented in local state contention (due to one single mutex state being used). Provider contention can support this.

3. Eager resumption: when a unit of work is either committed or destroyed, if it has interrupt stashes on the same node, it will immediately and unconditionally resume the interrupt stash with the earliest priority.

    Problem: As with any kind of eager parallel algorithm, this is not work-efficient due to non-level-ordering. During a single commit or a destruction, multiple interrupt stashes may be resumed at once without proper level-ordering.

    Motivation: Because it is hard to achieve level-ordering during resumption. The biggest problem is self-shadowed interrupt stashes where the interrupt stash is in a child node with the parent occupied by work from the same lane as the interrupt stash. This could happen if the interrupt happens via provider-consumer mechanism. No work-efficient top-down visit strategy can find a self-shadowed interrupt stash. By introducing extra lane markers, this could be solvable, but produces a very twisted and hard-to-reason execution history. 

### Dynamic subscription is BAD
Consider the extreme case of suspended rebuild. Prior to the rebuild, no one knows if the rebuild will succeed or suspend, so at first we must treat this rebuild as a authentic sync rebuild that will has the highest priority, so any provider update that will affect it will be interrupted. As soon as it is suspended, it will have to go into a race-to-commit model with any contending provider update, since it is neither appropriate to interrupt a provider update for a god-knows-how-long sync suspend (maybe it is appropriate after all?), nor treat sync suspend like a trash. As soon as the suspended rebuild gets polled again, it will become the highest priority again, until it suspend for a second time. This roller-coaster state migration is really really something we do not want to see.

Fixed subscription allows us to only focus on new subscriptions from inflating.

Static subscription is the best. Since it implies all subscriptions were done before any build could start, thus making suspended sync work easy to handle (all possible suspense happens after the subscirptions were resolved). We just need to focus on async inflating.

~~Let's try static subscription first.~~

Static subscription causes big waste for implicit animations!!!!! Since most of the times they do not need the subscription. To cancel the subscription would require a different widget type which breaks reconciliation.

Declarative subscription (dynamic but pre-calculated).

#### How to avoid waste on implicit animations?
1. Parent 
    1. use_state(Last command position, last command time, last static position, current position)
    2. If current position != last command position, return ChildVariant::ActiveChild
        1. ActiveChild
            1. consumed_types = \[Time\]
            2. new_position = interpolate(last static position, last command position, animation time, time - last command time, curve)
            3. use_effect(|position, active| if active {set_current_position(new_position)} else {set_last_static_position()}, \[new_position, true\])
    3. If current position == last command position || last command position == None, return ChildVariant::StaticChild
        1. StaticChild
            1. consumed_types = \[\]
            2. use_effect(|position, active| if active {set_current_position(new_position)} else {set_last_static_position()}, \[current_position, false\])

### What about provider: declarative or dynamic?

Declarative if we want to provide set_state as a provider. Since a mutable provider always comprises of a minimum of two layer of widgets. (We could )
We really need all mainline nodes to have a universal secondary_root_count state storage. The easiest way to do that is by having a elementcontextnode for every mainline node. Considering InflateSuspended state, that would mean either we require calling provider before the first suspend point (which is highly unlikely in normal code patterns), or a declarative provide.

This decision would really have profound design impact, since it affects ElementContextNode design.

Decision: declarative 
## Root Unit of Work
### Origin Roots
#### Top-level Roots
Definition: Roots of a lane that are not shawdowed by another higher root from the same lane.


### Spawned Roots
Definition: A unit of work can spawn another root unit of work deep down the tree.

### Execution Scope


### Two batch execution strategies
Current decision: eager batch dispatch.
#### Eager batch dispatch
If a top-level root node is unoccupied, then we immediately start active execution of the root work.

If along a tree path there exist top-level root A and top-level root B, where the priority of A's lane is higher than B. Then during dispatch, we visit through A (meanwhile trying to start active work in A) and still try to start active work in B. Even if later A's work may spawn into B and interrupt B's work.

Advantage:
1. When A's work visits through the MaxCR under A, it does not need to yield the subtrees below back to scheduler to examine the presence of other lane. In fact, we do not need a subtree yielding mechanism!
    1. Because other lanes would have already visited through A's MaxCR during their dispatch and are already executing.
2. All node either has an active current work, or they don't have any work (including backqueued) at all, which is easier to reason with.
3. If we are lucky, A's MaxCR may not contain B, and thus in the end A and B do not interfere. We get a higher throughput in this manner.
4. We don't even need a tree walk, we can just identify all top-level roots by lane marking and dispatch them right away.
5. Continue-work operation is naturally eager. (Or eager is naturally suited for dynamic root spawning)
#### Conservative batch dispatch
We only start a top-level root work when both the node is unoccupied, and the node's nearest ancestor work has yielded to the scheduler or is non-existent

If along a tree path there exist top-level root A and top-level root B, where the *priority of A's lane is higher than B*. Then during dispatch, we only try to start active work in A, and place a backqueue work of B in B. When A's work has visited through A's MaxCR, it will yield subtrees below the MaxCR to the scheduler. If they contains B and there is no other active work shadowing B, we try to start the backqueued work in B.

If along a tree path there exist top-level root A and top-level root B, where the *priority of B's lane is higher than A*. Then during dispatch, we visit through A during downwalk phase and try to start active work in B. Then during upwalk phase we try to start active work in A.

For multi lane scenario and considering interference from previous dispatched work, the above principle is generalized into:
1. Downwalk phase
    1. Record encountered ancestor lane work priority (including previous dispatches)
    2. If found a top-level root from the target lanes (including those from previous dispatches)
        1. If the priority is higher than the highest encountered priority, mark this node for possible active execution.
        2. Otherwise mark for backqueue
    3. Downwalk until we found all top-level roots of our target lanes
2. Upwalk phase
    1. Try to start active execution or backqueue according to mark

Disadvantage:
1. It will allow node to be in a backqueued state, where there is work in backqueue, but no active current work. This won't happen in the eager strategy.
    1. If there are bugs in scheduler, then a batch can be starved indefinitely in backqueue.
2. If a high-priority batch with a high-in-the-tree root hangs itself or become very slow, all other async batches will be blocked because the hanged batch does not yield. (Severe!)
3. We could lose some throughput for lanes with actually non-overlapping MaxCRs, despite we are more work-efficient now.
4. Dispatch process will be more complex, despite we only need a single dispatch every frame.
5. It can spawn various yield-subtree-to-scheduler requests, clogging up scheduler.
6. We haven't dive into interferences between yield from previous dispatch and a later dispatch. (Though the proof may not be very hard)
7. Then there comes the problem of continue-work. 
    1. In the example of A and B, if A has lower priority than B, then how should we repond when A spawned a consumer work at C below B?
        1. If we put a backqueue work at C. 
            1. It is entirely possible that B has already yielded subtree including C back to the scheduler, and the scheduler has finished processing this yield before we backqueued C. Then C will be lost. (Fatal)
        2. If we try to start active work at C.
            1. It is not work efficient anymore. Since B could be executing slowly and its MaxCR can contain C.
            2. It pretty much looks like as if we are doing eager batch dispatch (Severe)
                1. Actually, handling dynamic root spawning would always look like the eager strategy, including handling interference with previous dispatches.
    1. Especially, under a relaxed lane marking consistency, there can be false positives for continue-work.

## Synchronized Operations From the LaneManager
1. 




# Dispatch
At each dispatch, the scheduler will ensure all dispatched batches have their top-level roots into executing state (either active or backqueued).

For each active work, it will try to visit all nodes inside the MinCR, when it visit through MinCR node and there are still MaxCR beneath, it will yield back to the scheduler to continue in the subtrees of MaxCR.


# Lock Guidelines
<!-- 1. Must release parent wip lock before acquire children wip lock -->



# About BuildResults caching
BuildResults cache should be destroyed when subtree is being released. In theory we can preserve some caches during the release. However, there are two major problems if we preserve the caches: 1. provider updates happens 2. Resumed build did not touch the cached node ever again and we would be unable to find and clear this cache during commit time.


# Provider update during building
1. During inflate
    1. During async inflate. The first inflate result will be store no matter what. The inflater will register and check the provider value version when writing into the output, and will launch an reinflate if found out-of-date value. The following provider update will trigger more reinflate based on the first inflate result.
        1. Deadlock concern: When the follwing provider update is fired, do not read the output. Discard it immediately.
        2. Cyclicity concern: How does the inflate job fires reinflate on itself. Can be managed.
        3. Level-ordering concern: Since the node is constructed from bottom-up. The lower node will fire a reinflate first.
    2. During sync inflate. It isn't possible to have a provider update while we are holding the scheduler lock. WRONG! it can happen if the inflate is suspended.
        1. Cyclicity concern: The task need to hold a weak reference to the node. Arc::new_cyclic will not work due to possible interrupt. We have to have a ephemeral state. We can guarantee no one will try to upgrade the weak reference as long as we as the global scheduler lock holder do not trigger any provider update. As soon as we found out the work has suspended, we replace it with an interrupted state.
            1. We can do it with a Arc::new_cyclic_async

Foundamental provide with regard to the previous model:
1. Provider update cannot immediately interrupt and occupy affected nodes. Because the provider update has to perform a tree walk to reach its destination in order to guarantee level ordering.
    We will do the update notification only at the commit time. And we only specifically target those uncommited consumers, forcing them to re-execute. The commited consumers will only work as secondary root spawner and tree walk indicator.

    Marvellous


# Summary of task structure and provider update strategy
## Provider update
There are two types of provider update strategy:
1. Pull-based. When writing into the output, the building task pulls the provider value and version, checks them against the version read during the build, and choose to re-execute themselves if stale values are found.
    1. Pull-based strategy has to transition to push in the end. Since it cannot infinitely pull forever.
    2. The transition phase would require synchronization with all providers.
    3. The transition phase is also fallible. Need to ensure no rebuild is triggered by push before the transition is confirmed a success.
    4. We have to either
        1. Lock all providers to prevent update. Instant dead lock bug. In fact we cannot hold more than one provider lock at the same time or it is dead lock. And the providers may be lockfree after all.
        2. Lock self to prevent rebuild. Self-referencing concerns.
        3. Simply allow false provider update after a failed transtion.
    5. Other problems:
        1. Less efficient.
        2. 
2. Push-based. The provider update notifies the building task.
    1. Require a reference to locate the building task.

Conclusion: for task that can produce a reference to locate itself, we use push-based strategy.

We found by using a non-blocking async inflating scheme, we can produce a reference to the node during async inflate.

We found by using the necessary InflateInterrupted state and `Arc::new_cyclic`, we can produce a reference to the node during sync inflate. And also guarantee no one can commit a provider update to trigger weak pointer upgrade, since we hold the scheduler lock ourself.

Temporary decision: Use push-based strategy for all.


## Scoped structure or detached structure for tasks
Should the parent task await for children tasks completion?
1. Async rebuilds: Naturally detached since could be aborted anytime. But can also be scoped if you neglect the aborted error.
2. Async inflate: Scoped if using a blocking inflating scheme. Detached if using a non-blocking inflating scheme.
Detached for async is still generaly better. Since
1. A child can always be aborted and the parent should not be affected
2. Child task can always be replaced after being aborted. The replaced one cannot be reached by any original parent scope. So scope is useless.
Desicion: Using detached structure for async work.

PROBLEM: When cancelling async inflates, we are unable to propagate the cancellation.

3. Sync work: Generally prefer to be scoped since awaiting for completion signal in a sync batch looks very stupid.

Bottom-line: syncness should never be exposed to top-level `Element` constructs.

1. Use syncness-dependent task structure
    1. By returning child futures back from top-level `Element` constructs. This is seriously a bad API design
    2. By hiding async detach semantics with a returned empty future so they both need to be awaited. This would require to somehow sumtyping the empty future with a join handle.
2. Use scoped structure.
    1. If we need efficient reinflate (with interrupting the first inflate possible). The actual inflate process would still be detached anyway.

Drawback of syncness-dependent task structure: has to return 

Temporary decision:




# The correctness of the algorithm
The algorithm follows the principle of: 
1. A subtree visit must visit all the subtree if not interrupted
2. Who interrupts a subtree visit at a node must continue visiting the whole subtree under that node.
3. Low priority batch can in no way block the execution of the high priority ones. 






~~Primary roots must be associated with a CommitBarrier~~


Secondary spawn process must be used while holding the global lock. Since it uses reference counting and cancelling would be a challenge, and also the marking propagation must be synced.



# Problem
https://react.dev/reference/react/Suspense#preventing-already-revealed-content-from-hiding
> A transition doesn’t wait for all content to load. It only waits long enough to avoid hiding already revealed content. For example, the website Layout was already revealed, so it would be bad to hide it behind a loading spinner. However, the nested Suspense boundary around Albums is new, so the transition doesn’t wait for it.