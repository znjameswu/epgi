Performance hazard
1. https://github.com/rust-lang/rust/issues/110734
    1. Effect: generate significantly worse assembly for common executor parallel spawn patterns
    2. Possible solutions: 
        1. De-optimize the code by introducing an extra filter combinator to allow the compiler see it.
        2. Add a patch feature for above operations
    3. Resolution 
2. https://github.com/rust-lang/rust/issues/121719
    1. Effect: PointerEvent, if designed as a union of struct types, will generate horrendous asm when accessing common fields


Code-style hazard
- Async captures too many witness type
- Rust Analyzer errorneously resolves `Option::take`. https://github.com/rust-lang/rust-analyzer/issues/14933
- No object safety when supertrait dependes on associated type. https://github.com/rust-lang/rust/issues/40533
    1. Effect: No longer able to hide virtual methods with associated types behind a private supertrait.
    2. Workaround: Replace associated types with generic traits
- Cannot use associated const as const generic parameters. In fact
    1. Effect: If we want to contain specialization flavor inside Render/Element trait instead of leaking into RenderObject/ElementNode, we have to use associated type as mark rather than associated const. 
        1. But associated type cannot have default value (yet), unlike associated const. 
        2. Associated const cannot have equality constraint (yet), sealing any loophole-probing effort to indirectly use an associated const to control the associated type.
- Unable to prove disjointness of types with different associated types https://github.com/rust-lang/rfcs/pull/1672#issuecomment-1405377983. Workaround exist but verbose
    1. Effect: Unable to directly go and write separate tree walk implementations for two different specialization flavors of render objects / elements. 
    2. Effect: Unable to directly create multiple template traits for render objects / elements based on HktChildContainer disjointness.
    3. Effect: Unable to use associated const for Select\*Impl. The trait has to be generic.
- Unable to specific equality contraints for higher-kinded types https://users.rust-lang.org/t/how-to-express-type-equality-constraints-on-a-generic-associate-type/
    1. Effect: In the bilateral impl-supertrait binding pattern between trait pairs of \* and Select\*Impl, sometimes the supertrait bound need to constrain associated type, and sometimes the associated type is generic, such as SelectLayoutImpl::LayerCache. We have to use a cumbersome solution of explicit Hkt types.



Architectural hazard
1. An expensive work item could potentially block every other tasks in the rayon scheduelr. https://github.com/rayon-rs/rayon/issues/1054

Workflow hazard
1. RenderElement::SUSPENSE_FUNCTION_TABLE will break rustdoc. Hence its doc was hidden. Reason unknown. Repulsive enough.