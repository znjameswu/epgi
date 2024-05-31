








### Rust Future footgun
`Waker::wake` could be called when the poll is about to returning a `Poll::Ready` https://github.com/rust-lang/futures-rs/issues/2852


# Suspend
1. Sync suspend is polled by working the sync batch (point rebuild). Sync batch will always commit before the current frame is ready, even if it still leaves behind several suspended widget.
    1. Therefore 
2. Async suspend is polled directly by the scheduler. ~~Async batch will only commit when the batch is finally complete and when the scheduler allows it to.~~

## Question: should the async batch poll the suspended widget left by sync suspend?
Since the polling async batch may not end up being committed in a timely fashion, we still need the sync batch to poll the same suspended widget.

Therefore, if we allow it, there would definitely be two pollers and we need `futures::Shared`. (Note: we have already used `futures::Shared`, because we need to let async build to have a independent copy of hook state aside from the original copy to enable cancellation)

If the future just becomes ready to produce a final value when the async batch is about to poll it, it is possible that the async batch commit the completed node ahead of the sync point rebuild begins.
1. We need to prevent the sync point rebuild from happening. 
    1. Solution: abort the waker from the previous sync polling (now inside the sync mainline states) during async commit, and deactivate or remove the waker from accumulated waker queue in scheduler.


### Can we just forbid it? Delay the async batch until the sync batch has resolved the final value?
It is also possible, the point rebuild will force abort all uncommitted async work anyway.

Decision: Allow async batch to poll a sync future hook. Abort the waker during async commit and deactivate the waker in scheduler waker queue.


## Waker behavior when committing a suspended async batch
Suspended async batch sometimes can get committed. Therefore, we have to convert async wakers into sync wakers.

1. Use `futures::Shared`. During commit, just directly clone the future and then poll it with a sync waker and abort the async waker and deactivate or remove the sc.