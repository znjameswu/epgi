# What is a reversible async task

Sometimes we would wish to cancel a task **and** revert all its effect such that the end state would look like as if the task was never executed. We cancel a task rather than wait for it because cancelling early can save system resources. We revert the effect from the cancelled tasks because it makes it easier to maintain the invariants of a complex system state.

Any task can be modeled as follows
```rust
let shared_state = SharedStateContainer::new();
spawn(async move {
    let s = pure_computation();
    let f: impl FnOnce(SharedStateContainer) = construct_effect(s);
    apply_effect(shared_state, f);
})
```
In order to create a reversible task, there are a few obvious things that go without saying:
1. `f` must be reversible. That is, there must exist an `f_inv` to reverse the effect of `f` on the shared state.
2. `shared_state` must be able to identify and track all tasks of interest. That is, at any given point in time, given a snapshot of `shared_state` and any task of interest `t1`, there must be a definite answer on whether `t1` has executed its effect on the `shared_state`. If there is such a state confuses us on whether `t1` has executed, then we have simply lost the control of the situation.

The first part of our job is to cancel the task. To obtain the ability to cancel a task asynchronously, one usually need solutions like `futures::future::Abortable` and calls `AbortHandle::abort()`. 

(Note runtimes do provide various type of cancallation functions on their `JoinHandle`s. However, `JoinHandle`s are only created after the task was spawned, and sometimes we need to await on them, and they are not clone-able, creating certain inconveniences. Threfore their non-blocking/synchronous abort APIs are usually much cumbersome, if not unimplemented, like `async-std`. Moreover, if we somehow gets hold of the `JoinHandle` in our abort routine, there would be a very easy class of solutions called "blocking abort" by simply using a blocking/asynchronous cancel on the handle itself. There is no need to discuss this type of solutions in detail.)

Since abort/cancel can come at any time, the main problem of effect reversal can be summed up as two categories.
1. Missing effect reversal. The effect has fired and we are unaware.
2. Efficiency problem. The effect was not fired and we queried the state too many times.

When we write the routine to cancel and reverse a task, we have several choices
1. Recurring routine. The routine contains a cycle whose termination is unrelated to the state of the specific task we need to abort, such as a periodic poll to the states. We do not discuss this design. And this design is very likely inefficient.
2. Single-shot abort.
    1. Blocking abort. "Blocking" here does not refer to a thread blocking. It means the routine is blocked by the progress of the target task. The obvious solution is to call the `abort`, await the handle for task return, reverse its effect if it was executed.
    2. Non-blocking abort. Although the routine may contains code pieces such as a contending mutex lock, it does not depend on the overall progress of the target task.

# Non-blocking abort
Since the routine is non-blocking, no part of it would have any guaranteed ordering with any part of the target task. The only two possible synchronization pairs are 1. the `abort` event and the target task await points, 2. accesses to the shared state. 

However, the `abort` call actually provides zero guaranteed syncrhonization with the target task. After the `abort` become visible on the task thread, it is possible that the task does not stop at any `await` point after that. This is because in order to stop the execution of the `Abortable` task, the poll must return at least one `Pending` after the `abort` become visible. All the `await` points in the task can be classified into two categories: 1. `await` an resource unrelated to the abort routing 2. `await` an shared results protected by some async synchronization primitive. The first category may always return `Ready` on polling. The second category always trace their dependence to locks and thread scheduling, which have no guaranteed order, therefore our target may always be so lucky to acquire the locks on every attempt and return `Ready` on polling. Therefore, it is always possible that the polling went straight through to completion and our `abort` was simply ignored. Therefore the only meaningful synchronization we have on hand is the access to the shared state. 

A special case that is worthing mentioning is the `yield_now` primitives provided by many executors. They should guarantee to always return `Pending` on the first polling, therefore establishing a reliable synchronization with our `abort`. We shall discuss it later.

By reading from the shared states, we can only have a single bit of information: whether the effect was executed on the shared state or not when the abort routine access the states. However, there are at least three scenarios of interest constructed by the abort and the shared state access: the task was aborted before the effect was executed, the task was not aborted before the exeuction and the effect execution happens before/after the read from the abort routine. So it is theoretically impossible to achieve task reversal by just reading from the shared states in the abort routine. We have proven that we *must* write into *some* shared states in our abort routine. Accompanying that we also would have to read that piece of state in our task. This sets our baseline for the most efficent possible implementation.

## Cooperative task cancellation for states protected by locks
A simple implementation arises using `AbortHandle::is_aborted()` API
```rust
// Variables shared by the abort routine and the target task.
let (abort_handle, abort_reg) = AbortHandle::new_pair();
let shared_states = Arc::new(Mutex::new(some_states()));


// Abort routine
abort_handle.abort();
let mut guard = shared_state.lock();
reverse_effect_if_present(&mut *guard);
drop(guard);


// Target task
spawn(Abortable::new(async move {
    let s = pure_computation();
    let f: impl FnOnce(&mut SharedState) = construct_effect(s);
    let mut guard = shared_state.lock();
    if abort_handle.is_aborted() {
        return;
    } else {
        f(&mut *guard);
    }
    drop(guard);
}, abort_reg))
```
You can take time to verify the correctness of memory ordering in this program. Since we actively checked a flag in our task, this is called cooperative task cancellation.

Compared to a minimal barebone code using `Mutex` and `AbortHandle`, we introduced zero extra variable, one extra write in our abort routine, one extra read and one extra branching in our task, and an extra capture (`abort_handle`) in our async move closure. Recallling that 

> ... We *must* write into *some* shared states in our abort routine. Accompanying that we also would have to read that piece of state in our task.

In order for the read to happen, we need a reference to the new pieces of state in our async closure capture. In order for the read to be effective, the control flow after that must depend on the read value somehow. If our task is returning empty `()`, then the minimal possible dependency on our read value that we can ever introduce would be an empty branching. And exactly an extra reference and an empty branching alongside a read is introduced in the the cooperative task cancellation algorithm. Therefore, we proved that the cooperative task cancellation *is* indeed one of **the most theoretically efficient algorithms** for a reversible task system whose shared state is protected by a lock.

## Solutions for states protected by atomic variables
One solution using cooperative task cancellation is as follows. Though the most theoretically efficient solution remains unknown.
```rust
// Variables shared by the abort routine and the target task.
let (abort_handle, abort_reg) = AbortHandle::new_pair();
let shared_states = Arc::new(AtomicUsize::new(0));


// Abort routine
abort_handle.abort();
atomic_reverse_effect_if_present(&shared_states, Release);


// Target task
spawn(Abortable::new(async move {
    let s = pure_computation();
    let f: impl FnOnce(usize) = construct_effect(s);
    atomic_apply_effect(&shared_states, f, Acquire)
    if abort_handle.is_aborted() {
        atomic_reverse_effect_if_present(&shared_states, Relaxed);
    } 
}, abort_reg))
```
Warning: If there will be aliased tasks in different point of time (such as a reused lane in lane marking), the delayed reversal in the target task WILL BREAK our program, since the OS may decide to interrupt our task for a year, right before the delayed reversal. Only a blocking abort or a mutex will save us.


## Solutions for states protected by RCU-style primitives
RCU-style primitives include widely used crates such as `arc-swap`

One solution using cooperative task cancellation is as follows. Though the most theoretically efficient solution remains unknown.
```rust
// Variables shared by the abort routine and the target task.
let (abort_handle, abort_reg) = AbortHandle::new_pair();
let shared_states = ArcSwap::from(Arc::new(some_states())));


// Abort routine
abort_handle.abort();
shared_state.rcu(|s| reverse_effect_if_present(s));


// Target task
spawn(Abortable::new(async move {
    let s = pure_computation();
    let f: impl FnOnce(SharedState) = construct_effect(s);
    shared_state.rcu(f);
    if abort_handle.is_aborted() {
        shared_state.rcu(|s| reverse_effect_if_present(s));
    } 
}, abort_reg))
```
If RCU has AcqRel ordering, then the above algorithm is correct.

Warning: If there will be aliased tasks in different point of time (such as a reused lane in lane marking), the delayed reversal in the target task WILL BREAK our program, since the OS may decide to interrupt our task for a year, right before the delayed reversal. Only a blocking abort or a mutex will save us.


## Solutions for states protected by non-blocking data structures
This would really depend on the exact memory ordering guarantee of the non-blocking data structures. No general solutions can be recommended other than using mutex-based cooperative task cancellation.

## Synchronization by `yield_now`
`yield_now` is a tricky primitive as we previously mentioned. Currently no generic `yield_now` solution is available and only native versions provided by each async runtime seem reliable enough to use (See https://github.com/smol-rs/smol/issues/43). Even then, most native versions do not seem to provide a solid guarantee and enough QA involvement. Meanwhile the official Rust RFC for the behavior model of `yield_now` is still unavailable. So, this method is generally not recommended.

We can insert `yield_now` points in our code to ensure abort of task at specific point. They would work similarly like the cooperative checking, despite acting by a different mechanism. For the example of async mutex protected state, we have.

```rust
// Variables shared by the abort routine and the target task.
let (abort_handle, abort_reg) = AbortHandle::new_pair();
let shared_states = Arc::new(Mutex::new(some_states())); // Async mutex


// Abort routine
abort_handle.abort();
let mut guard = shared_state.lock().await;
reverse_effect_if_present(&mut *guard);
drop(guard);


// Target task
spawn(Abortable::new(async move {
    let s = pure_computation();
    let f: impl FnOnce(SharedState) = construct_effect(s);
    let mut guard = shared_state.lock().await;
    yield_now().await;
    apply_effect_on_inner_states(&mut *guard);
    drop(guard);
}, abort_reg))
```
Since we awaited inside the lock critical section, this limits us to only use async mutexes. And the `yield_now` must successfully return a `Pending` and terminate the task if the abort becomes observable. Thus this algorithm strongly depend on the implementation quality of the native `yield_now`s.



# Some common mistakes

## "Magic of global locks"
It is easy to think that acquiring a globally unique lock can somehow works its magic despite a lack of cooperative checking. THIS IS WRONG!

A naive attempt may produce the following code.

```rust
// Variables shared by the abort routine and the target task.
let (abort_handle, abort_reg) = AbortHandle::new_pair();
let shared_states = Arc::new(Mutex::new(some_states())); // Async mutex
let HOLY_GLOBAL_SCHEDULER_SOMETHING_LOCK = Arc::new(Mutex::new());


// Abort routine
let holy_guard = HOLY_GLOBAL_SCHEDULER_SOMETHING_LOCK.lock().await;
// A long chain of operation...........
abort_handle.abort();
// Even more long chains of operation...........
let mut guard = shared_state.lock().await;
reverse_effect_if_present(&mut *guard);
drop(guard);
drop(holy_guard);


// Target task
spawn(Abortable::new(async move {
    let s = pure_computation();
    let f: impl FnOnce(SharedState) = construct_effect(s);
    let holy_guard = HOLY_GLOBAL_SCHEDULER_SOMETHING_LOCK.lock().await;
    let mut guard = shared_state.lock().await;
    apply_effect_on_inner_states(&mut *guard);
    drop(guard);
    drop(holy_guard);
}, abort_reg))
```
Note that we *must* keep the lock acquisition order the same, otherwise it would instant become a deadlock bug. With deadlock in mind, our ways of using the global lock are surprisingly limited. In the code example above, some may hope that contending on the global lock can somehow prevent the task from entering its critical section after the abort has been called. However, as we discussed before, we have absolutely zero guarantee regarding how threads, tasks, and locks are scheduled. It is possible our OS arbitrarily decide to preempt our task during `construct_effect` for a year and then after the year it find the global lock is available, therefore ignoring the abort command from a year earlier. 

We have proven something like a cooperative checking is always a good choice under this scenario.

## Cooperative checking outside the critical section
Rust recommends API design of hidding mutex operations inside struct and methods(Todo: find source). Therefore, adding a foreign flag in an API that perfectly conceals the lock operations seems to break this elegancy. It may be tempting to move the cooperative checking outside the critical section and even out of the lock-concealling method itself. However, this simply produces wrong results for reversible tasks.
```rust
// Variables shared by the abort routine and the target task.
let (abort_handle, abort_reg) = Abortable::new_pair();
let shared_states = Arc::new(Mutex::new(some_states()));


// Abort routine
abort_handle.abort();
let mut guard = shared_state.lock();
reverse_effect_if_present(&mut *guard);
drop(guard);


// Target task
spawn(Abortable::new(async move {
    let s = pure_computation();
    let f: impl FnOnce(SharedState) = construct_effect(s);
    if abort_handle.is_aborted() {
        return;
    } else {
        let mut guard = shared_state.lock();
        apply_effect_on_inner_states(&mut *guard);
        drop(guard);
    }
}, abort_reg))
```

This suffers the same problem. The OS may abitrarily decide to hang our task thread for a year between the checking and the lock operation. Rendering this checking useless.

# Cooperative task cancellation for tree structures

Suppose the shared state is distributed in a tree structure and each tree node is individually protected by a mutex. And suppose in this case, both the task and the abort routine have taken the form of a top-down *recursive* tree visit. There are two common data structure design for this:
    1. The task remains being monolithic and is wrapped in a single, outermost `Abortable` and controlled by a single `AbortHandle`. Then the implementation remains the same as the previous cooperative task cancellation: A single cooperative flag and cooperative checking on the same flag before each mutex write.
    2. The "task" is fractal and comprises of multiple small tasks, each working on a single tree node with individual abort handles. In this case, the abort handles themselves will be distributed across the tree as well. A efficient solution for this scenario is:
        1. Each small task stores their abort handles inside the node mutex they visit
        2. Each small task use the abort handle stored in their *parent* node as their cooperative checking flag.
        3. Abort routine takes out each abort abort handle as it visit node mutexes, and calls abort before at anytime before visiting their children.

        The reason to use parent abort handles as cooperative flag is because, before unlocking the mutex in the current node, the only shared handle that the abort routine can acquire is the one inside its parent node, unless you somehow stored all the abort handles globally. And the cooperative checking needs 1. shared abort handles 2. calling abort before entering the critical section.

        This is exactly how diced implemented its parallel build system with partial interruptible rebuilding.

# Several code guidelines for this project
1. Whenever an async work writes into a non-exclusive mutex, a cooperative flag must be used.
2. Whenever an async work calls the global scheduler, a cooperative flag must be used.