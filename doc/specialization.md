
Element & Render Specialization Design
==============

# Problem Statement
Element & Render have different flavor/variant that have significant behavior differences and type contracts, **while** they also share a great amount of similar behavior similarities. 
- Element provide value vs no value
- Element has a render object vs no render object
- Dry layout vs "wet" layout
- Non-layer vs layer
- Layer has a cache vs no cache
- Layer is orphan vs not


If we construct different generic node type for all those behavior combinations, there would be too many boilerplate (2^2 ElementNode type, 2^4 RenderObject types). This calls for a solution of specialization.

# Problem analysis
These specializations actually have very different effects.
1. Pure behavior changes. 
    1. Their change is strictly confined in procedural code implementations.
    2. Example: dry layout vs wet layout, provide value vs no value. 
2. Type layout changes.
    1. They also change the presence of some fields in their nodes.
    2. Example: RenderObjectElement needs to store render object pointer. LayerRenderObject needs to store layer cache. Cached layer needs to store extra composition cache.
3. Type contract changes.
    1. They adds or removes some type contracts in their nodes.
    2. "Add contract" example: LayerRenderObject has restrictions on its Protocol types. (This is trivial)
    3. "Remove contract" example: non-RenderObjectElement is restricted to have a single child with the same protocol through. RenderObjectElement removes this restriction.
    4. Other implicit "add contract" examples: Layerred render object should be able to be cast into AnyLayerRenderObject, etc. 


# Solution candidates
## Rust nightly specialization feature.
We don't speak nightly

## Merge all possible method in a single trait, and specialize by associated const bools.
A humoungous trait with the union of all methods from all possible specializations. And the specialization methods not chosen should be implemented with `panic!` or `unreachable!`

1. Ugly and cumbersome. It creates confusion when methods have default impl: no clear distinction between whether a method is impl-ed by default or unreachable due to specialization.
2. It cannot express type layout changes on its own. Unless the user is very proficient and knows all the contract between all type parameters.
3. It cannot express type contract changes on its own. Unless effectively using the "Select-trait" pattern below.


## Associated enum with function pointers
Description: Create a function pointer struct to immitate a trait. At the declaration site, store different versions of those function pointer structs inside an associated const enum in the trait. At the use site, match enum to retrieve specialized method from function pointers.

1. It cannot express type layout changes.
2. Function pointers cannot be generic. Most of time it fails to express type contract changes, because a type contract can be instantiated to infinitely many type relations, due to the presence of generics.
3. Cumbersome for users.

## Associated type with unifying trait bound
```
trait Render: Sized {
    type IsLayerRender: IsLayerRender<Self>;
}

trait LayerRender: Render {
    type CachedComposition;
}

trait IsLayerRender<R: Render> {
    type CachedComposition;


}

struct True;

struct False;

impl<R> IsLayerRender<R> for True where R: LayerRender {
    type CachedComposition = R::CachedComposition;
}

impl<R> IsLayerRender<R> for False where R: Render {
    type CachedComposition = ();
}
```
1. It can express type layout changes.
2. It is not sufficient to express some type contract changes.
## "Select-trait" pattern
Creates


    