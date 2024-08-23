- Use task-local storage to pass environmental variables such as is_self_sync to enforce certain invariants during debugging.
- ~~NOW: Async suspense~~
    - Async reconcile has_mailbox_update optimization
- Layout intrinsics
- Pointer add/remove
- Optimize query interface probe failure cost
- Find a way to remove the cloning cost inside `Text` widget without disrupting its signature
- `use_state_ref` plagued with ownership lifetime issues when multiple hooks are used.
- implicit animation hook should use a better signature to allow external triggering.
- MSRV 1.75 for RPITIT in Element consumer type

TODOs for other crates:
