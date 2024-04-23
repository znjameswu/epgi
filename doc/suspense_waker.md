








### Rust Future footgun
`Waker::wake` could be called when the poll is about to returning a `Poll::Ready` https://github.com/rust-lang/futures-rs/issues/2852


# Suspend
1. Sync suspend is polled by working the sync batch (point rebuild). Sync batch will always commit before the current frame is ready, even if it still leaves behind several suspended widget.
    1. Therefore 
2. Async suspend is polled directly by the scheduler. Async batch will only commit when the batch is finally complete and when the scheduler allows it to.

## Question: should the async batch poll the suspended widget left by sync suspend?
Since the polling async batch may not end up being committed in a timely fashion, we still need the sync batch to poll the same suspended widget.

Therefore, if we allow it, there would definitely be two pollers and we need `futures::Shared`.

If the future just becomes ready to produce a final value when the async batch is about to poll it, it is possible that the async batch commit the completed node ahead of the sync point rebuild begins.
1. We need to prevent the sync point rebuild from happening. 
    1. Solution: abort the waker from the previous sync polling (now inside the sync mainline states) during async commit


### Can we just forbid it? Delay the async batch until the sync batch has resolved the final value?
It is also possible, the point rebuild will force abort all uncommitted async work anyway.
