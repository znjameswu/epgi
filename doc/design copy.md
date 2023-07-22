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
Bind by element node



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



Job Yielding by stashing or by waiting mechanisms (such as CondVar or tokio::Notify)?

Decision: yield by waiting


## Do we need a parent pointer inside element?
Yes


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



## Protocol as generic parameter or associated type?
This is a stupid question. It MUST be associated type.


# New part

### Concepts
