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

# Disjointness based on associated type trick
The original disjointness trick (https://github.com/rust-lang/rfcs/pull/1672#issuecomment-1405377983) doesn't work well between crates

Imagine we use associated type to allow downstream crate to provide a general impl for an upstream trait
```rust
// upstream crate
trait SelectImpl {
    type I;
}
trait P {}
trait PBy<I> {}
impl<E> P for E where E: SelectImpl + PBy<E::I> {}
```
```rust
// downstream crate
struct I2<E>(PhantomData<E>);
trait P2 {}
impl<E> PBy<I2<E>> for E where E: P2 {}
```
The downstream impl violates the ordered orphan rule!

However, a small modification would align us well with the orphan rule ordering.
```rust
// upstream crate
trait SelectImpl {
    type I;
}
trait P {}
trait ImplP<E> {}
impl<E> P for E where E: SelectImpl, E::I: ImplP<E> {}
```
```rust
// downstream crate
struct I2<E>(PhantomData<E>);
trait P2 {}
impl<E> ImplP<E> for I2<E> where E: P2 {}
```

# Inheritance emulation

Inheritance is very useful tool in UI tools. It is very natural to abstract away the base trait which is often too verbose and too hard to impl, and let users impl a more specific trait, and "inherit" the all the behaviors available on the base trait.

Indeed, rust allows "impl trait on trait" pattern to allow this behavior, but only in a very limited way. Rust's type system enforces a strict orphan rule. Generally speaking, Rust only allows for one child trait for one base trait. So the inheritance relation is strictly a linear list (rather than a tree), if you can't prove the disjointness between child traits. 

Linear inheritance is of little use for UI library which can create a vast number of child traits based on different aspects of assumptions. We need a inheritance tree rather than a linear inheritance list.

Now we can construct disjoint generic traits by using different associated types, it is natural to extend it to emulate a inheritance tree.

Three fundamental elements for a minimal inheritance tree, or the process that happens when you use an abstract class in java:
1. Superclass Reification. You fill in several required abstract method required by a subclass, the subclass pass it upwards to fill in all abstract method in the superclass, thus the superclass may actually become complete and operable.
    1. The reification needs to prove the disjointness between subclasses. Any ambiguity will not be allowed in Rust. This can be achieved by the disjointness by associated type trick.
2. Subclass Induction. The subclass acquires all the implementation and interface from the superclass.
3. Recursion in both of the above process (Optional). This allows the tree to be multi-layered, instead of just two layer.

We then check to see if they are possible to emulate in Rust

## Compile-time recursion in Rust
Fundamental elements for a recursion:
1. Recursion requires a recursive type relations. 
2. Recursion requires a base case, or a stop point to stop the recursion when you reached the base trait.

This would instantly causes conflicting implementation problem, because stop point and normal case has two recursion behavior.

### Basic recursion emulation
Luckily, Rust's type system actually has a negative-reasoning provision in its orphan rule that enables such recursion with a stop point.

Related: https://aturon.github.io/blog/2017/04/24/negative-chalk/

Related: https://github.com/rust-lang/rfcs/blob/master/text/1023-rebalancing-coherence.md
```rust
// Impl target
trait ImplBase {
    fn foo_impl(&self); // The signature is notional. The Self receiver type actually has no use. The next code snippet shows a correct signature.
}

trait ImplBaseBySuper {
    type Super: ImplBase;
}

// Recursion
impl<I: ImplBaseBySuper> ImplBase for I {
    fn foo_impl(&self) {
        I::Super::foo_impl(self)
    }
}

struct BaseImpl;

// Stop point
impl ImplBase for BaseImpl { // Note this does not conflict with the previous impl block
    fn foo_impl(&self) {
        // Base logic
    }
}
```

Note how the two impl blocks does not conflict with each other, since the orphan rule restricts the only possible impl between `ImplBaseBySuper` and `BaseImpl` can only be present in the current crate, and thus the negative reasoning works. And `BaseImpl` becomes the stop point.

Note, however, this recursion is *one-way*. That is, given the superclass, derive the subclass. To achieve recursion usable in inheritance, we need *two-way*. We need a way to derive superclass from subclass recursively (previously called "reification").

### Recursion for reification
Reification is the process of taking a `SubTrait` impl and translate it into `SuperTrait` impl. We can already write out what such reification process would look like in a impl

```rust
// Prototype, not working
impl<ISub> SuperTrait for ISup
where
    ISub: ImplBaseBySuper<Super = ISup>,
    ISub: SubTrait,
{
    // .....
}
```
This has several implications.
1. `SubTrait` can only appear in downstream crate.
2. In downstream crate, `ISup` can never be a local type, even if it is a generic type (because the type constructor is not local).
    1. (FATAL) By orphan rule, if `ISup` is generic, because `ISup` is not local type constructor, then no free type parameters may appear in `ISup`
    2. In downstream crate, `SuperTrait` has to be generic and contain at least one local type as its parameter before any type parameters appear.
3. Since reification needs to be invoked somewhere, and we certainly want to retain the very basic ability to directly reify the root superclass. Then, we have a contending impl, also in the definition crate:
```rust
impl SuperTrait for ISup 
where
    ISup: ExtraSuperTrait
{
}
```

This conflict, with the orphan rule's restriction on how `ISup` could be designed, blocks any attempt at making reification recursive.

We preserve other failed approaches at the end of this article.

## What happens to inheritance if we cannot recurse
We have a two-layer inheritance structure.

This is identical to what an interface/trait system would look like. Inheritance is dead.

## Dropping the idea of mergine inheritance discriminant with specialization
This causes rust-analyzer running way too slow. Not happy with the design.

### Other failed approaches (for record only, ignore explanation)

```rust
// In downstream crate
impl<ISub> SuperTrait for ISup
where
    ISub: ImplBaseBySuper<Super = ISup>,
    ISub: SubTrait,
{
    // .....
}
```
```rust
impl SuperTrait for ISup 
where
    ISup: ExtraSuperTrait
{
}
```
We can see a conflict since disjointness during reification cannot be proved.

We can resort to our disjointness by associated types trick. We need to make `SuperTrait` and `ISup` both generic and associate with a type as discriminant, suppose `Element`. And `Element` happens to be able constrain our `ISub` parameter.

After a bit of renaming and accomodating the newly-introduced generics, we have
```rust
trait Element {
    type Impl: ImplElement<Element = Self>;
}
trait ImplElement {
    type Element: Element;
}
trait ImplElementBySuper {
    type Super: ImplElement;
}
impl<I: ImplElementBySuper> ImplElement for I {
    type Element = <I::Super as ImplElement>::Element;
}

trait SuperTraitFor<E> {}
struct ISup<E>(PhantomData<E>);
impl<E: Element> ImplElement for ISup<E> {
    type Element = E;
}
trait ExtraTraitForSuper {}
struct LocalE;

trait SubTrait {}

impl<E: Element<Impl = ISub>, ISub> SuperTraitFor<E> for ISup<E>
where
    ISub: ImplElementBySuper<Super = ISup<E>>,
    ISub: SubTrait,
{
    // .....
}

impl<E: Element<Impl = ISup<E>>> SuperTraitFor<E> for ISup<E> where E: ExtraTraitForSuper {}
```
In theory, we proposed the disjointness by the negative reasoning that `ISup<E>` does not implement `ImplElementBySuper`. This should be supported by orphan rules, since:
1. In downstream crate, `ISup` is not local, therefore `ISup<E>` is not `LT` type. Therefore the whole `ISup<E>: ImplElementBySuper` is forbidden no matter what the `E` is
2. In upstream crate, there is simply no `ISup`
3. In the current crate, we did not write any relevant impl.

But rust's type solver failed to prove this and we have a conflicting implementation. A disappointment since chalk does not strictly conform to RFC 1023. We conclude that chalk cannot perform negative reasoning on associated types (as evidenced by various other failed approaches).

Even if chalk can perform the negative reasoning of `ISup<E>: !ImplElementBySuper`. A more challenging scenario follows, where `ISup` is not the "most" super supertrait, and instead do have `ISup<E>: ImplElementBySuper<Super = ISupsup<E>>`, and we need a more enhanced version of negative reasoning of `ISup<E>: !ImplElementBySuper<Super = ISup<E>>`. Whoaaa, this extra layer of associated type reasoning maybe too much for chalk.





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

Because we need to convey the self type into the helper trait, given that using generic trait is not possible, we can either use generics in individual trait methods or a generic struct.

Generic trait: doesn't seem possible?

Generic struct: Looks more and more resembling the Select\* trait pattern

Works

> Notes after ditching inheritance emulation:
> 
> Since inheritance emulation is no longer pursued, we naturally rollback from our generic struct decision, and revert back to generic trait. This would make life easier for the trait solver.



Failure: Inheritance emulation, where the discriminating associated type is shared with specialization emulation, cannot be achieved due to Rustc's inability to prove disjointness by orphan rules in certain cases. https://github.com/rust-lang/rust/issues/123450.
```rust 
pub trait Foo {}

pub trait ImplBy {
    type Impl;
}

pub trait FooBy<I> {}

impl<T> Foo for T
where
    T: ImplBy,
    T: FooBy<T::Impl>,
{
}

struct Bar;
impl ImplBy for Bar {
    type Impl = ();
}
impl Foo for Bar {}
// conflicting implementations of trait `Foo` for type `Bar`
// downstream crates may implement trait `FooBy<_>` for type `Bar`
```

Explanation: Inheritance emulation's exploit on orphan rule depends on the root type to not implementing discriminating trait (`Impl\*BySuper`), while specialization requires

Another attempt