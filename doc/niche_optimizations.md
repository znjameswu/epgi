1. Optimize for single child work. `Reconciler::into_*`
    1. Optimize for single child sync work. `await` instead of `spawn_scoped`
2. Optimize for fixed number child. `Element::ChildIter`
3. Optimize for non-suspendable child during sync work. `Element::ReturnResult : MaybeSuspend<_>`
    1. Directly commit consumers instead of putting into uncommited consumers since no rollback is possible.
    2. Skips book-keeping for suspendable.
4. Optimize for single-child tree walk. `await` instead of spawn
5. Optimize for sync work
    1. Scoped structure for tasks & commit on-site. No need to wait for signal
    2. Element state clone elison. 
        1. A sync task directly takes the state out and then return after finished
        2. A non-suspendable sync task avoids clone the state in all its path
6. static scheduler instead of Arc.

