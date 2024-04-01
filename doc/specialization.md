Specialization and Inheritance
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


TODO

# Inheritance emulation

Inheritance is very useful tool in UI tools. It is very natural to abstract away the base trait which is often too verbose and too hard to impl, and let users impl a more specific trait, and "inherit" the all the behaviors available on the base trait.

Indeed, rust allows "impl trait on trait" pattern to allow this behavior, but only in a very limited way. Rust's type system enforces a strict orphan rule. Generally speaking, Rust only allows for one child trait for one base trait. So the inheritance relation is strictly a linear list (rather than a tree), if you can't prove the disjointness between child traits. 

Linear inheritance is of little use for UI library which can create a vast number of child traits based on different aspects of assumptions. We need a inheritance tree rather than a linear inheritance list.

Now we can construct disjoint generic traits by using different associated types, it is natural to extend it to emulate a inheritance tree.

Three fundamental elements for a minimal inheritance tree:
1. Disjointness of child traits. We need to prove no child trait under a common parent trait can overlap. This can be achieved by the associated type trick.
2. Recursive resolution. When implementing a descendant trait, the compiler have to recursively trace upward to at least know which base trait is targeted, and generate implementation for the base trait.
    1. Recursion requires a recursive type relations. 
    2. Recursion requires a base case, or a stop point to stop the recursion when you reached the base trait.

Rust's type system actually has a provision in its orphan rule that enables such recursion with a stop point.
```rust
// Impl target
trait ImplBaseBySuperOrSelf {
    fn foo_impl(&self); // The signature is notional. The Self receiver type actually has no use. The next code snippet shows a correct signature.
}

trait ImplBaseBySuper {
    type Impl: ImplBaseBySuperOrSelf;
}

// Recursion
impl<T: ImplBaseBySuper> ImplBaseBySuperOrSelf for T {
    fn foo_impl(&self) {
        T::Impl::foo_impl(self)
    }
}

struct BaseImpl;

// Stop point
impl ImplBaseBySuperOrSelf for BaseImpl { // Note this does not conflict with the previous impl block
    fn foo_impl(&self) {
        // Base logic
    }
}
```

Note how the two impl blocks does not conflict with each other, since we can prove `ImplBaseBySuper` is not implemented for `BaseImpl`. And `BaseImpl` becomes the stop point.

This technique, when combined with disjointness techniques from our specialization experiment, however, no longer works.

```rust
// Target
trait Base {
    fn foo(&self);
}

// Marker trait for disjoint associated types
trait ImplBase: Sized {
    type Impl: ImplBaseBySuperOrSelf<Self>;
}

// Connect target with helper trait
impl<T:ImplBase> Base for T {
    fn foo(&self) {
        T::Impl::foo_impl(self)
    }
}

// Target trait's helper trait, exist on associated types.
trait ImplBaseBySuperOrSelf<T> {
    fn foo_impl(value: &T);
}

trait ImplBaseBySuper<T> {
    type Impl: ImplBaseBySuperOrSelf<T>;
}

// Recursion
impl<I: ImplBaseBySuper<T>, T> ImplBaseBySuperOrSelf<T> for I {
    fn foo_impl(value: &T) {
        I::Impl::foo_impl(value)
    }
}

struct BaseImpl;

// Error: conflicting implementations of trait `ImplBaseBySuperOrSelf<_>` for type `BaseImpl`
// downstream crates may implement trait `ImplBaseBySuper<_>` for type `BaseImpl
impl<T> ImplBaseBySuperOrSelf<T> for BaseImpl { 
    fn foo_impl(value: &T) {
        // Base logic
    }
}
```
The stop point no longer works, because we cannot prove that `ImplBaseBySuper<T>` is not impl-ed for `BaseImpl` for every possible `T`, since `T` could be a user-defined type, even though `ImplBaseBySuper` is a private trait. The problem is caused by using generic trait to associate target types with associated types.

Because we need to convey the self type into the helper trait, given that using generic trait is not possible, we can either use generic trait method or a generic struct.

Generic trait: doesn't seem possible?

Generic struct: Looks more and more resembling the Select\* trait pattern

