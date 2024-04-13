1. Use task-local storage to pass environmental variables such as is_self_sync to enforce certain invariants during debugging.
2. NOW: Design BuildContext and effects
3. ~~NOW: Tree walks~~
4. ~~NOW: Requeue async tasks~~
5. ~~NOW: Interrupts~~
6. NOW: Async suspense
7. ~~NOW: Provider write~~
8. ~~NOW: Cancel skip subtree~~
9. NOW: WorkContext updating
10. NOW: Make sure lane marking and dispatching checks unmountednedss
11. NOW: async overdue blocking
12. ~~NOW: Use "descendant + self" mark combination instead of "subtree + self" combination. The former provides an unmarking oppurtuny during the return phase of tree walk.~~
13. Zero size container reconcile optimization (skip updating children, etc.)
14. Render text
15. Pointer add/remove
16. ~~Hit test transform design~~
17. Optimize query interface probe failure cost
18. ~~Cache adopted layers in composition cache for hit test~~


TODOs for other crates:
1. ~~Default for vello::Transform~~
2. ~~Vello API design issue~~