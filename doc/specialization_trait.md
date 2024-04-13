Current specialization implementation could cause severe confusion or even trait solver ICE, due to how the traits cyclic depends on each other. As a response, this repository cuts cycles and linearizes traits as much as possible. For cycles that are not reducible, this repository forces an explicit ordering of those trait by associated bounds. This documents provides details on the above process.

# Theory
We employ a much more restrictive well-formedness requirement than what was allowed in chalk. In this project, we have at least three large scale type prototype failed due to chalk panics out (a.k.a., Internal Compiler Errors). Aside from ICEs, there are various other (a dozen maybe) smaller prototypes (some of which doesn't make into commit) failed due to chalk's non-conformance to Rust standard coherence model (RFC 1023). This project has deemed the chalk behavior to be not trustworthy at this complexity scale, and comes up with a much more simple and restrictive coherence model *for the most complex parts of specialization implementation*.

The following description welcomes any type-theory expert to modify its language.

## Acyclic trait impl system.
Definition:
1. For any two different traits, A and B, they will have one of three possible relations: A is descendant of B, A is ancestor of B, A and B are independent.
2. Ancestral relation is *transitive*. If A is adescendantof B, and B is descendant of C, then A is descendant of C.
3. A supertrait declaration with a format of `trait A: B {}` or `trait A where Self: B {}` designates A as descendant of B.
4. A impl block with a format of `impl<T> A for T where T: B {}`  designates A as descendant of B.
5. An associated item with a format of `impl<T> A for T {fn a() where Self: B  {}}` is allowed for any trait A and B theoretically, but with some restrictions on clause explained below to prevent us from ICEs.
6. (Obviously) there should be no two pieces of code should produce conflicting descendant relations



## Mental model
1. We are effectively creating a **directional acyclic graph** (DAG) of traits. Let's assume upper traits are ancestors and lower traits are descendants.
2. You can only write impl blocks for lower trait by using where clauses containing upper trait.
3. If you use associated type bound clause, then that trait in bound must climb up the DAG via a different route without running into the trait you are impl-ling.
4. A most common case for associated type bound is using "sibling" traits inside.

### More finer-grained verison (Clause-based)
1. For any two different type bound clauses, CA and CB, they will have one of three possible descendant relations: CA is descendant of CB, CB is descendant of CA, CA and CB are independent.
2. Ancestral relation is strictly *transitive*.
5. `trait A where CB {}` is allowed if and only if clause `Self: A` is descendant of clause `CB`.
6. `impl<T> A for T where CB {}` is allowed if and only if clause `T: A` is descendant of clause `CB`.

Additional restrictions on associated clauses:
1. If an associated clause and a impl-block clause share the same ancestral clause which bounds a type to a trait with an associated type, make sure both clauses contains the same explicit associated type equality constraints! Failure to do so can easily trigger ICEs.
    - Example, `impl<R: Render<Impl=Self>> XX for I { fn a() where R: FullRender<Impl=Self>; }`. You need to specify both `Impl=Self` on them. Both or none.
2. Try avoid using descendant clause. Try replace with all other sibling clause that, when combined with the self clause, can infer to the given descendant clause.

However, usually the trait-based model is enough to explain what we are doing, though not exactly precise in some cases.

## Rationale
This allows an acyclic type checking for the trait solver. When checking impl-block clauses, the checker can only move upward in DAG, ultimately guaranteeing it to **halt**. 

See Rust well-formedness check https://rust-lang.github.io/chalk/book/clauses/wf.html#impls

This have several implicaions:
1. `trait A: B {} impl<T> B for T where T: A {}` is strictly prohibited in this model, despite being a perfectly legal Rust pattern.
2. This model does not prohibit the trait aliasing technique of `trait A: B {} impl<T> A for T where T: B {}` (shortnoted as `trait A = B`).
3. Generally speaking, you cannot rely on **any** functionalities from "more specific" traits to appear in the where clauses when you are implementing "less specific" traits.
4. Although you can't use "more specific" traits in your where clauses, we do not prohibit you to use them inside function bodies if Rust automatically infers it. As in the trait aliasing example `trait A: B {} impl<T> A for T where T: B {}` (shortnoted as `trait A = B`), Rust will infer methods of A inside B's method bodies. 

# Element traits
Analysis:
1. Full function element requires to select specialization for reconcile functionality.
2. `ImplReconcileCommit` needs to write into `ElementNode`.
3. `ElementNode` needs to select specialization for whether it has `RenderObject`

Solution: Break element implementation into multiple stages. 
1. Stage 0 (`ElementBase`): Any specializations can be implemented by targeting `ElementBase`, as long as their signature does not require `ElementNode<E>` to have a layout.
2. Stage 1 (`Element`): `ElementNode` now has a memory layout because certain specializations are selected in this stage. Any specializations can be implemented by targeting `ElementBase`, as long as their signature does not require `ElementNode<E>` to be attachable into tree (e.g. have `ChildElementNode` trait implemented).
3. Stage 2 (`FullElement`): Full functionality of `ElementNode` exists and all specializations are selected.
```
┌───────────┐                       
│ElementBase│                       
└───┬───────┴──────┐                
    │              │                
┌───▼───────────┐ ┌▼──────────┐     
│ImplElementNode│ │ImplProvide│     
└───┬───────────┘ └┬──────────┘     
    │              │                
┌───▼───┐◄─────────┘                
│Element│ Enables ElementNode layout
└───┬───┘                           
    │                               
┌───▼───────────────┐               
│ImplReconcileCommit│               
└───┬───────────────┘               
    │                               
┌───▼───────┐                       
│FullElement│                       
└───────────┘                        
```

# Render traits
Analysis:
1. `ImplPaint` on some path could need to convert `Arc<RenderObject<R>>` into `ArcLayerRenderObject`, requiring full render object functionalities plus layer functionality.
2. `ImplHitTest` needs to convert `Arc<RenderObject<R>>`, requiring full render object functionalities
3. `RenderObject` needs to select specialization for whether it is a layer.
4. `ImplHitTest` and `ImplComposite` both needs to select specialization for its orphan behaviors.
5. Layer functionalities needs to ensure it has `ImplComposite`

Current Solution:
1. Similarly break render implementation into multiple stages.
2. 

```
┌──────────┐                                   
│RenderBase│                                   
└───┬──────┴────────┬─────────────────┐        
    │               │                 │        
┌───▼────────────┐ ┌▼─────────┐ ┌─────▼───────┐
│ImplRenderObject│ │ImplLayout│ │ImplComposite│
└───┬────────────┘ └┬─────────┘ └─────┬───────┘
    │               │                 │        
┌───▼──▲────────────┘                 │        
│Render│                              │        
└───┬──┘ ┌───────────────┬────────────┤        
    │    │               │            │        
    ├────┼─────────────┬─┼──────────┐ │        
    │    │             │ │          │ │        
┌───┴────▼─────┐ ┌─────┴─┴───┐ ┌────▼─▼──┐     
│ImplMaybeLayer├─►ImplHitTest├─►ImplPaint│     
└───┬──────────┘ └─────┬─────┘ └────┬────┘     
    │                  │            │          
┌───▼──────▲───────────┴────────────┘          
│FullRender│                                   
└──────────┘                                   
```