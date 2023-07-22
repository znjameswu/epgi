Performance hazard
1. https://github.com/rust-lang/rust/issues/110734
    1. Effect: generate significantly worse assembly for common executor parallel spawn patterns
    2. Possible solutions: 
        1. De-optimize the code by introducing an extra filter combinator to allow the compiler see it.
        2. Add a patch feature for above operations
    3. Resolution 


Code-style hazard
1. Async captures too many witness type
2. Rust Analyzer errorneously resolves `Option::take`. https://github.com/rust-lang/rust-analyzer/issues/14933
3. No object safety when supertrait dependes on associated type. https://github.com/rust-lang/rust/issues/40533
    1. Effect: No longer able to hide virtual methods with associated types behind a private supertrait.
    2. Workaround: Replace associated types with generic traits


Architectural hazard
1. An expensive work item could potentially block every other tasks in the rayon scheduelr. https://github.com/rayon-rs/rayon/issues/1054