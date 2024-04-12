

# Notes about orphan layer redesign

# Old design
Previously, the orphan layer carries all the following responsibilities
1. Attach to parent render object as child
2. Registering self as orphan during paint
3. Attach to adopter layer as adopted child layer
4. Compsite with adopter and receive hit tests from adopter.

This causes the adopter canvas to be different from parent canvas.

Example scenario: a 3D object want to have a 2D selection handle in the final UI overlay.

Under the old design where an orphan layer contains the selection handle:
1. The orphan layer has to have a 3D parent protocol in order to be inserted into the 3D world
2. The orphan layer has to have a 2D child protocol to host selection handle
3. The orphan layer should register itself as orphan layer while being painted by its parent.
4. The orphan layer should actually consider itself to be a 2D render object in order to be adopted by its adopter
    1. And should handle 2D hit test instead of 3D hit test.

This has more consequences:
1. This makes this orphan layer to be a 3D `ChildRenderObject`, but a 2D `ChildLayerRenderObject` and `ChildRenderObjectWithCanvas`
    1. Which means to have to specify a new concept: `AdopterCanvas`. So that all layers should identify themselves to be `ChildLayerRenderObject` of the `AdopterCanvas` instead of `ParentProtocol::Canvas`
2. This makes this orphan layer to composite to 2D canvas, rather than to its parent 3D canvas
    1. Which makes `Composite` and `HitTest` traits to rely on `AdopterCanvas` type, which is unknown prior to select `ORPHAN_LAYER` variant in element impl. 
    2. We have to either bake `AdopterCanvas` into `Composite` and `HitTest` traits (By generic traits), or duplicate the traits into non-orphan/orphan version.
3. This makes `CompositionResults` to also depend on `AdopterCanvas` causing great clutter in type signatures.

This makes this design very very intrusive and propagate `AdopterCanvas` clutter all over the codebase. This makes composite process especially hard to reason with, and makes composition phase a torture to work on.

# Other designs that were dropped
1. Orphan layer drops the responsibility to attach to adopter and compsite/hit-test with adopter. Instead, it directly sends it children to the adopter. This design should rather be called as "relinquishing parent"
    1. Not good. Now the transfered render object do not necessarily has a layer between them and adopter. However, painting requires a layer to provide a `PaintContext` to paint into. Therefore, now the child transfer MUST happen before painting. Otherwise, painting into their relinquishing parent's `PaintContext` has no use. However, the orphan child cannot be known in advance before we actually executed user-supplied paint logic. Now we destroyed our beautiful and parallel-friendly pipeline.

# New design
The orphan layer drops the responsibility to directly attach to parent render object. Instead, it attach to a canvas adapter layer which attaches to the original parent render object *if the original parent canvas is different from the adopter canvas*.

This has no overhead in same-canvas orphan layer. While it do introduces extra layer in cross-canvas scenarios, we can assume this to be the minor case and accept it as a slow path.

Now the adopter canvas will always be the same as the direct parent canvas. `AdopterCanvas` concept can be eliminated along with all API bake-ins.

## Rationale
This was actually the oldest design from the very beginning of designing orphan layer system. However, at that time, we did not have accurate estimate of the scale of API disruptions cause by it, and decided that cross-canvas overhead should be avoided. Hence comes the old design. Now we are simply fed up with it.

