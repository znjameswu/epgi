## Job
All immediate state changes from an event defines a job. Performing a job means apply the state changes, rebuild and reconcile affect element, trigger appropirate effects, etc.

Each job will experience the following lifecycles:
1. Requested. The job ID and type is created by the event under processing.
2. Ready. The event has finished delivery and the job is ready to be performed.
3. Executing. If the lane is not sync, the batch may be suspended or aborted by the sync lane. When aborted, all reconciliation results are discarded.
4. Completed. The batch reconciliation is completed and the new element nodes are ready to be commited.
5. Commited. The updates are commited into the element tree and the effects are processed.

## Batch / Lane
Jobs are executed in batches. Each batch will be exclusively assigned a lane when executing. There are total of 64 lanes in 64-bit systems and 32 lanes in 32-bit systems.







## Event
Event consist of following categories:
1. User event
2. Timer event
3. External event
4. Transition event (Transitions inside a transition will be considered as the same transition as the parent)

# Job and Rebuilds
The build phase contains the execution of two types of workloads: jobs and point rebuilds.

## Job
A "job" refers to all immediate ("immediate" means not blocked by unresolved suspensions) state changes and subsequent UI changes caused by a event. Performing a job means applying state updates, building widget, reconciliation, triggering effect, spawning more work, etc.

Any two jobs have one of the following relations:
1. Independent: The two jobs spawned units of work that did not overlap in terms of modified element node.
2. Conflicting: The two jobs spawned units of work that made modifications to the same element node, but not on the same state hook.
3. Sequenced (Entangled): At least one root unit of work from two jobs made modifications to the same state hook in the same element node. Their sequence is determined by:
  1. If the origin frame of the job is earlier than the other, then it is sequenced before the other.
  2. If the origin frame of the two jobs is the same, and one job is sync and the other is async, then the sync job is sequenced before the async job.
  3. If the origin frame and synchronicity is the same, then the sequence order is determined by the source event sequence.

Note: Sequenced jobs can be detected prior to job execution. However, many conflicting jobs can only be detected during execution.

## Point Rebuild
A "point rebuild" is a type of workload that rebuilds a (potentially suspended) element node without applying state changes. They are usually caused by the resolution of a previously pending future that causes a suspense.

## Unit of Work
Pending changes to a specific Element. That includes applying state updates, building widget, reconciliation, triggering effect, spawning more work, etc.

A job consist of several root units of work. A point rebuild usually consists of one root unit of work. A work may spawn more work in child Element nodes.

Work can be classified by two method

By their configurations (semantics):
1. Inflate (Creates a new element)
2. Rebuild (Works on an existing element)
    1. Update (Changes widget)
    2. Refresh (Does not change widget)

By their runtime behavior: (This is how source code is organized)
1. NewInflate (Creates a new element)
2. Rebuild (Works on an existing element)
    1. Reinflate (Element is suspended in the last inflate/re-inflate attempt)
    2. Rebuild (Other)
    

Update must be the direct children of another update or a refresh. In a running system, inflate must the be the children of another work.

There is also "unmount" that can be called as a type of work. However, "unmount" takes almost zero time in building and most of its computation happens during commit. So, unmount is considered a commit-time effect, rather than a official type of work.

## Batch / Lane
There are total of 64 lanes in 64-bit systems and 32 lanes in 32-bit systems.

There are two types of lanes:
1. Sync lane. There is one and only one sync lane. The sync batch will always be executed synchronously.
2. Async lane.

Each job will be assigned with a lane prior to execution. Sync jobs events will always be assigned the sync lane. 

All point rebuilds will always be assigned to the sync lane. 

Work in the same lane within a given scheduler frame will be executed in one batch. 

# Tree Structure
## Path
## Subtree

# Scheduling Overview
## Scheduler main loop
1. The accumulated events during the last frame time is collected.
2. Events are delivered and thus root units work of jobs are created. 
3. The scheduler will start the execution phase. This phase will block all async batch commits until this phase ends.
    1. We can potentially commit completed async batches here. This would potentially skip a few pale events. However, this introduces additional interactivity racing. (Decision: Commit async batch here. Helps to reduce livelocks caused by new events.)
    2. Batching and lane marking are performed. If there is a change in lane for a job, then the job is first aborted then re-enqueued.
    3. The sync batch and async batches that are ready and not conflicting with higher-priority batches start executing, aborting all conflicting async batches it encounters.
    4. Sync batch finishes execution.
<!-- . Sync execution phase ends. Now async batches are allowed to commit. -->
4. Commit whatever async batches that has finished.
5. Render the tree.
  1. Layout
    1. After layout, we can start dispatching events for follow-up frames
  2. Paint
    1. After painting, we are ready to start working on the next frame.
  3. Composition
<!-- ## Lifecycle of a sync job
1. Requested. The job ID and type is created by the event under processing.
2. Ready. The event has finished delivery and the job is ready to be performed.
3. Executing. If the lane is not sync, the batch may be suspended or aborted by the sync lane. When aborted, all reconciliation results are discarded.
4. Completed. The batch reconciliation is completed and the new element nodes are ready to be commited.
5. Immediate work committed. The updates are commited into the element tree and the effects are processed.
6. Delayed work committed.
7. Done.

## Lifecycle of an async job
1. Requested. The job ID and type is created by the event under processing.
2. Ready. The event has finished delivery and the job is ready to be performed.
3. Executing. If the lane is not sync, the batch may be suspended or aborted by the sync lane. When aborted, all reconciliation results are discarded.
4. Completed. The batch reconciliation is completed and the new element nodes are ready to be commited.
5. Commited. The updates are commited into the element tree and the effects are processed.
Before committing, async jobs ca -->

FIFO mode
```
VSYNC                                             VSYNC                                             VSYNC
  │                                                 │                                                 │
──┤   ┌───────┬────────┬───────┬───────────┐        │                                                 │
  │   │       │        │       │           │        │                                                 │
──┴───┤ Build │ Layout │ Paint │Composition│        │Event Proc Delay                                 │
      │       │        │       │           │        │   │                                             │
──┬───┴───────┴────────┴───────┴───────────┘   ────►│   │◄────                                        │
  │                                                 │   ├───►start_new_frame                          │
  ├─────────────────────────────────────────────────┤   ├───────┬────────┬───────┬───────────┐        │
  │                  Event Collection               │   │       │        │       │           │        │
  ├────────────────────┬────────────────────────────┴───┤ Build │ Layout │ Paint │Composition│        │
  │                    │    Event Dispatch & Markup     │       │        │       │           │        │
  │                    ├────────────────────────────┬───┴───────┴────────┴───────┴───────────┘        │
  │                    ├───►prepare_next_frame      │                                                 │
  │                                                 ├─────────────────────────────────────────────────┤
  │                                                 │                  Event Collection               │
  │                                                 ├────────────────────┬────────────────────────────┴──
  │                                                 │                    │    Event Dispatch & Markup
  │                                                 │                    └────────────────────────────┬──
  │                                                 │                                                 │
```

Mailbox Mode
```
                         Event Proc Delay
                               │   │
                           ───►│   │◄───
                               │   │
──┐   ┌───────┬────────┬───────┼───┴───────┐
  │   │       │        │       │           │
──┴───┤ Build │ Layout │ Paint │Composition│
      │       │        │       │           │
──────┴───────┴────────┴───────┼───┬───────┘
                               │   │
  ┌────────────────────────────┤   ├───────┬────────┬───────┬───────────┐
  │      Event Collection      │   │       │        │       │           │
  └────────────────────┬───────┴───┤ Build │ Layout │ Paint │Composition│
                       │ Disp&Mkup │       │        │       │           │
                       └───────────┴───────┴────────┴───────┴───────────┘

                               ┌────────────────────────────┐   ┌───────┬────────┬───────┬───────────┐
                               │      Event Collection      │   │       │        │       │           │
                               └────────────────────┬───────┴───┤ Build │ Layout │ Paint │Composition│
                                                    │ Disp&Mkup │       │        │       │           │
                                                    └───────────┴───────┴────────┴───────┴───────────┘
```

## Timing Constraints 
1. Event dispatch must start after the layout phase of the previous frame
2. Build phase must start after the painting phase of the previious frame
  1. Event collection should end after the painting phase of the previous frame (Since we cannot start the build phase anyway, so why not collect more events?)
3. Build phase must start after the completion of event dispatch of the current frame
4. Rasterization phase must start after the completion of rasterization of the previous frame

# Scheduling Gurantees:
1. Units of immediate work spawned directly from a sync job will be completed atomically. That is, updates from a sync job that are not partitioned by future hooks will always be presented wholly in any frame.
2. Units of work from async jobs will be completed atomically. That is, async jobs will never present a partial updates.
3. A sync job will always batch with all jobs that are sequenced-before it in the sync batch. All sync jobs will be batched in the sync batch. The remaining sequenced async jobs will be batched. 
4. Sync jobs are always batched in one single sync lane and always finish uninterrupted in the current frame.
5. The ordering of units of work inside a single batch will follow the order of job creation, except for delayed work. 
6. (Goal) The scheduler will try to execute as many non-conflicting jobs in parallel as possible.
7. For jobs spawning in the same frame, any async job is guaranteed to be committed no earlier than any sync job.

# Scheduler Implementation









Questions
1. What will happen, when an async job is blocked by a resolution, while a new sync job gets entangled with the async job?
    1. Promote the async job as a sync job
        1. How would we implement this? Discard
    2. Break up the async job and only promote the entangled part, leaving an orphaned async job.
2. (Cont. 1) What will happen when an async job that is susceptible to encounter futeure resolution gets entangled with a sync job.
    1. Discard async job and promote it to sync job.
        1. Will we reuse the previous resolution results?
            1. If the future is produced during the build, then we will have no choice but to invoke the future twice (Acceptable)
3. Why not commit async batches at any time? Because commits will modify element tree, but the event distribution requires a steady render object tree.
4. Why not commit the async batches during idle time and distribute the event at one time? Because event distribution requires up-to-date render tree structure and layout information, but commits will expire the layout information.


## How scheduling is performed
We use two types of scheduling to maximize parallizable batches in execution.
1. Planed execution
2. Oppurtunistic execution