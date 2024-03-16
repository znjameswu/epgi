https://docs.flutter.dev/ui/interactivity/gestures#gesture-disambiguation

https://developer.apple.com/documentation/uikit/touches_presses_and_gestures/handling_touches_in_your_view

Flutter's design

# PointerAdd and PointerRemove
- They are dispatched directly by PointerRouter without hit-test. (no hit-test pointer event)
- PointerRouter manages subscriber from the tree
- MultiDragGestureRecognizer and Multi-tap would subcribe to the PointerRouter
    - GestureRecognizer::addPointer -> MultiDragGestureRecognizer::addAllowedPointer -> PointerRouter::addRoute

Pointer event distribution is a transient selective process, whereas focus is a persistent selective process.

Focus should be an element-level property (Realy? Why?). However, ReadingOrderFocusTraversal requires layout results (or to be more precise, bounding boxes) to be involved.



# Touch event and primary buttons
https://github.com/flutter/flutter/issues/30454

https://github.com/flutter/flutter/pull/30339

# Pointer capture
Flutter defaults to pointer capture for all pointer events. UIkit also seems to be doing so.

W3C requires explicit setPointerCapture execpt for direct manipulation elements, https://w3c.github.io/pointerevents/#dfn-implicit-pointer-capture

## Should we impl pointer capture by default or explicit optional pointer capture?
Pointer capture by default is easier to implement, but I can't think of a way to enable selection across widgets.

If pointer capture is enabled by default, then when a selection gesture moves out of a TextView, the following pointer event won't be delivered to its sibling text views. Then we are unable to select into sibling views.

Decision: explicit pointer capture and release. Buttons should capture pointer on pointer down, and release pointer when pointer up. Selectable widget should release pointer when pointer is out of region

Problem: pointer event may not start with pointer down. drag selection across a button

We need a explicit "Keep"/Null variant on pointer capture response. All pointer should respond with a "Keep" to a pointer move event. And buttons should only capture pointer on pointer down, not pointer move.

# Drag direction and affine transform
Affine transform may transform widget axis to non-orthogonal directions on screen. This is problematic for us to determine whether a drag is main axis or cross axis (horizontal or vertical).

Flutter use a extremely weird and almost certainly buggy strategy to determine two-axis drags. (https://github.com/flutter/flutter/blob/9e1c857886f07d342cf106f2cd588bcd5e031bb2/packages/flutter/lib/src/gestures/monodrag.dart#L402) 

# Flutter gesture recognizer call flow
Example: VerticalDragGestureRecognizer

RawGestureDetector -> Listener::onPointerDown -> RawGestureDetectorState::_handlePointerDown -> GestureRecognizer::addPointer -> OneSequenceGestureRecognizer::addAllowedPointer -> OneSequenceGestureRecognizer::startTrackingPointer -> PointerRouter::addRoute && OneSequenceGestureRecognizer::_addPointerToArena

PointerRouter -> DragGestureRecognizer::handleEvent -> DragGestureRecognizer::_checkDrag (VerticalDragGestureRecognizer::_hasSufficientGlobalDistanceToAccept) -> OneSequenceGestureRecognizer::resolve -> GestureArenaEntry::resolve -> GestureArenaManger::_resolve -> GestureArenaMember::accept

or DragGestureRecognizer::handleEvent -> DragGestureRecognizer::_giveUpPointer -> OneSequenceGestureRecognizer::stopTrackingPointer & DragGestureRecognizer::resolvePointer;

## Flutter drag recognition criteria
DragGerstureRecognizer::_globalDistanceMoved += globalDelta * sign(globalDelta.dot(axisVector))

If DragGerstureRecognizer::_globalDistanceMoved > 18 then accept


Hittest

GestureBinding::_handlePointerEventImmediately -> WidgetsFlutterBinding::hitTestInView -> RendererBinding::hitTestInView -> RenderView::hitTest

### Example: MultiTapGestureRecognizer (Tap never declares victory but rather wait for all competitors to withdraw)

GestureRecognizer::addPointer -> MultiTapGestureRecognizer::addAllowedPointer -> _TapGesture -> gestureArena.add(MultiTapGestureRecognizer) & add_TapTracker::startTrackingPointer -> PointerRouter

PointerRouter -> _TapGesture::handleEvent -> update info. Never declares victory, only checks when pointer up or informed victory.

-> MultiTapGestureRecognizer::acceptGesture -> _TapGesture::accept -> _TapGesture::_wonArena = true & _TapGesture::_check

_TapGesture::_check -> MultiTapGestureRecognizer::_dispatchTap

 
## Flutter MultiTouch
### Example: ScaleGestureRecognizer

GestureRecognizer::addPointer -> ScaleGestureRecognizer::addAllowedPointer -> OneSequenceGestureRecognizer::addAllowedPointer -> OneSequenceGestureRecognizer::startTrackingPointer 

PointerRouter -> ScaleGestureRecognizer::handleEvent -> ScaleGestureRecognizer::_advanceStateMachine -> OneSequenceGestureRecognizer::resolve -> GestureArenaEntry::resolve -> GestureArenaManger::_resolve -> GestureArenaMember::accept

## PointerHover?
1. Scrollable::handleHover <- MouseRegion
2. RenderPointerListener::handleEvent

# Pointer enter and exit
MouseTracker::_handleDeviceUpdateMouseEvents





# Gesture Recognition Systen Design
EPGI's gesture recognition system is heavily influenced by Flutter. However, the design is modified and adapted to better fit the overall architecture in EPGI and Rust style guidelines. Compared to Flutter, this systems is more explicit, less recursive (self-referencing) in terms of implementation style.

## Major challenges
1. Multi-pointer gesture recognition
    1. A gesture recognizer might simultaneously subscribe to multiple pointers. And when the recognizer requests resolution / withdraws / handles victory / handles defeat due to one pointer event from one pointer, it will cause **cascade effect** on other pointer arenas.
    2. This requires a multiplex between pointer ids and gesture recognizers. And mostly importantly, a **stable** identity of each gesture recognizer.
    3. More alarmingly, this stable identity of each recognizer is hard to emulate with a stable gesture recognizer team.
    4. This stable identity can be solved by a hit test entry with recognizer type id tuple.
2. sweep-hold-release alternative for double taps
    1. Can we simply keep the last tap timestamp inside our gesture recognizer when we are ordered to clean-up? 
        1. NO, NO, NO. We have to notify the arena to update it state in case the withdrawal produces a winner.
            1. Actually rather than notifying, we can simply poll every frame.
3. External notification to the arena
    1. The arena has to take external notifications. For exmaple, when a widget gets rebuilt and one of its gesture recognizer gets destroyed/replaced, the arena must be notified and evict the recognizer, since there could be only one other recognizer that could win by default after this eviction.
        1. What about a delayed eviction? The outdated recognizer only gets evicted when processing next relevant pointer event.
4. Gestures recognition is inherently time-dependent.
    1. Flutter and JS has built-in async timer support. While we must impl an equivalent timer service just for gesture recognition.

## Gesture Arena, Gesture Recognizer Teams, and Gesture Recognizers
There exists a unique, application-wide *gesture arena* for each pointer.

*Gesture recognizers* are arena participants that intends to claim ownership of the current pointer interaction and to interpret it as a gesture.

*Gesture recognizers* from the same render object always team up before entering gesture arena, thus forming *gesture recognizer teams*. The render object that hosts those gesture recognizers / gesture recongizer teams is called hence *gesture recognizer team object*. With a render object, a team may nest under other teams as a team member. (That is to say, teams are fractal, and the arena itself behaves like a global team.)

The arena for a given pointer becomes active on corresponding pointer down event and collects competing *gesture recognizer team objects* in the hit-test results. Then it will feed all remaining teams with pointer event from the given pointer if a event comes in.

### Confidence, Resolution, and Withdrawal
When feed with a pointer event, a gesture recognizer and a gesture recognizer team will respond with a float point value called confidence. Confidence represents the certainty level and eagerness of the recognizer / team on its intepretation. The confidence reported by a team is usually the highest confidence reported from its members.

1. If confidence $\geq 1.0$, it means the recognizer / team is very certain its intepretation is correct, and request the arena to immediately resolve a winner.
2. If confidence $\leq 1.0$, it means the recognizer / team request immediate withdraw and no longer receiving any further event feed.
3. If confidence $\in (0,1)$, it means the recognizer / team is still interested in the interaction.

If a render object is unmounted while the arena is active, the team is also considered to have withdrawed.

If all members of a team have withdrawed, the team withdraws.

There are four possible outcomes for an arena:
1. Resolved. At least one team has requested for resolution, and the team with the highest confidence is declared winner. If multiple candidate reports the same confidence, then we compare the render object's depth in tree (the deeper the better), then its hit-test order (the upper the better).
2. Defaulted. No team has requested for resolution, but only one team has remained in the arena. The team is declared winner by default.
3. Empty. No one is declared winner because the pointer up event has fired because no team has remained in the arena. Either there is no team to begin with, or all remaining teams withdrawed at the same time.
4. Swept. The pointer up event has been fired and no team requested for resolution, nor report an inconclusive result. The team with the highest confidence is declared winner.

From there on, the arena will continuously feed pointer events to the winner if there is one, until the pointer up event fires and the arena closes and becomes inactive.

### Competing, Cooperative, Hereditary Teams
Once a team is declared winner, the team also needs to decide internally on its winner member (recognizer / child team). 

For single-member teams, this is trivial. For multi-member team, there are multiple strategies.
1. Competing (Most common). The team acts like an arena before choosing a member as winner. Either a member requests for resolution, or progresses by default, or waiting until the team becomes empty and closed. Therefore, even when arena has declared a team as winner by default, the team may still not having a winner inside and thus no gesture event is fired.
2. Cooperative. The team immediately elects the member with the highest confidence as the winner.
3. Hereditary. The team immediately choose its leader as the winner. The leader is chosen according to a specified hereditary order.






# The lack of a uniform position transformation between cavnases
EPGI emphasizes on canvas-agnostism, it does not naturally assumes the canvas to be an Affine2D planar canvas like Flutter does.

For exmaple, we should expect a 3D scene to be seamless embeded inside an Affine2D canvas, without losing any common functionalities of hit test and extensible with pointer events dispatch and gesture recognition, if downstream library author desires so.

This would create much restrictions on our design. We would lost a so-called `localToGlobalTransform` concept in Flutter, where in any position in tree, we can expect to access the transformation between our local coordinate space with the actual device screen.

For a better example, let's consider the following complex exmaple.
1. An Affine2D scene with user interactivity is present.
2. The Affine2D scene was embeded on a planar surface in a 3D scene.
3. The 3D scene is embedded with curvilinear projection (say five-point fish-eye projection) onto the 2D device screen.
4. Now we wish to interact with the inner most scene with user gesture.

This exmaple brings the following challenges
1. Curvilinear projection nullifies any hope to map the coordinates using linear algebra
2. Even worse, curvilinear projection has a bounded range (or in math terms, non-surjective into the full 2D plane). That is to say, if we pick an arbitray point on screen, the preimage (reverse-transformed object in 3D) may not even exist. Even if we successfully hit-test back through the projection at the inital pointer event, then as the pointer continues to move, it may then leave the projection range entirely. Then we would never know how to express the moved delta in local coordinate space.
3. Curvilinear projection has non-uniform local distortions. Any delta calcuations have to be specific with the pointer position. Which means for each pointer movement, we have to re-perform a delta calculation.

On top of this diffult scenarios, other challenges exist as well
4. No unified position data type. Good luck using a `(f32, f32)` as hit test position.
5. Hit test coordinate is no longer point-like. In a 3D canvas that can be embeded in 2D canvas, its hit test coordinates are straigt lines rather than points.
6. Hit test coordinate translation with type-safety.
7. Moving render objects. The render objects may be moving by itself at the same time we are recognizing gestures. 
    1. Flutter uses PointerRouter to transform following pointer event to gesture recognizers. The PointerRouter uses static transform information registered during initial gesture recognizer tracking, which itself comes from initial dispatched pointer down event, which itself comes from initial hit-testing results. As a result, Flutter would dispatch the event as if the render object does not move.
    2. Implementing Flutter's solution would be cumbersome in our canvas-agnostic design.
    3. Remember, we also  use semi-absolute coordinates in regards to canvas. Flutter's design would not work in the first place.


# A modular and type-safe hit-test interface design
## Problem statement
1. Hit-test is usually serving two purposes: pointer event dispatch and gesture recognition (though two sound similar, they are actually two different systems).
    1. Flutter specializes pointer event dispatch by HitTestTarget interface. 
    2. Flutter also has an explicit global singleton gesture arena (By explicit, we mean widget authors can directly access it globally).
2. However, hit-test in its essence should be a general widget selection mechanism, not tied to pointers and gestures.
3. Moreover, for exotic canvases, there may simply not exist either pointer or gestures (e.g. terminal UI canvas). There is no fundamental reason to special-case them. Off-loading them to a modular approach keeps canvas-agnotism.
4. Interface design is split into two parts: how to express it in the interface of `Render`, and how to encode hit-test data structures.

## Challenges
1. Different canvas has difference hit-test position primitives.
    1. This makes a specific "capability" for hit-test handling to be a higher-ranked (generic trait). I.e., if a render object can handle pointer event, the trait it needs to satisfy is `PointerEventTarget<HitTestPosition>`. Note how `PointerEventTarget` itself is not a concrete trait nor type.
    2. Therefore it makes it tricky to filter render objects with specific capabilities out of all hit-tested candidates, since all candidates have different hit-test poisition types and a higher-ranked trait does not have a concrete type construct.
2. Redundancy in transform stack
    1. Each hit-test candidate has to record its own transform stack from screen, which always overlaps with the transform stack of its parent. This is fine. However, when we are iterating through all hit-test candidates, this could bring a O(N^2) complexity in recording all transform stacks. (We could accept O(N^2) complexity for filtered candidates, but never for unfiltered, initial hit-test results. There could be hundreds of entries in it.) (Well, we might be able to accept it?)
        1. Especially when actually transforming screen coordinate to hit positions, if not handled well, there could be huge repetition of computation and results in O(N^2) complexity. While in reality, there only needs O(N) transform computation at max.
3. *Absorbing rule
    1. In flutter, despite the wild hitTest interface design, the actually implemented hit-test code strictly follows pointer absorbing rules: If one child declares they have absorbed the pointer event, the parent should not continue to hit-test any other children below this child.
    2. This ~~significantlly~~ somewhat bypassed a lot of branches.
    3. This rule contradicts with our generic hit-test design
        1. How do you know what capability have we asked for, before letting one child just simply absorb it? The child could be specialized in another capability and therefore should not absorb if we asked for a different capability.
    4. Temporary decision: Not baking absorbing rule into interface design. Defer it to pointer event dispatcher.
4. Use snapshot transforms or up-to-date transforms
    1. This is relevant for gesture recognition. Since a gesture recognition takes time and need to route many pointer events, if the render object changes position during the process, should we transform the following pointer events according to the new position or according to the snapshotted position recorded during hit-test?
    2. Flutter use pure snapshot transforms. 
        0. `GestureBinding::_handlePointerEventImmediately -> WidgetsFlutterBinding::hitTestInView -> RendererBinding::hitTestInView -> RenderView::hitTest`
        1. `HitTestResult::addWithPaintTransform` and `HitTestResult::addWithPaintOffset` to endocde in `HitTestEntry::transform`
        2. `GestureBinding::dispatchEvent -> ... -> GestureRecognizer::addPointer -> ... -> PointerRouter::addRoute`
    3. Decision: snapshot transforms.



## Design
1. Bind the hit position with the render object to form a single polymorphic object.
    1. Since from the scheduler's point of view, we only have on-screen positions and have no idea about each candidate's canvas type, if we ever want to enumerate/store the candidates, we either bind and seal the render object with its corresponding hit position together and dyn cast them away, or we use `mut Box<dyn Any>` to pass in hit position, which is not type-safe.
    2. Choice: bind in a transparent temporary tuple, or cache inside the render object?
        1. Decision: A transparent temporary tuple `RenderObjectHitPositionPair`
        2. Rationale: 
            0. Hit-test is supposed to be pure without effects. We should allow concurrent hit-testing. Caching is dirty.
            1. For a "dumb" render object which does not interact with hit test system anyway, we shouldn't cache anything inside. Caching would require mutex locking which is expensive. We have to make our interface tell those dumb objects apart.
            2. Even for a non-"dumb" render object, it could have nothing to do with the current hit business (i.e. a pointer event handling render obejct during gesture arena iterations), we should avoid mutex locking in this scenario. It is impossible to prevent this while being concurrent.
            3. Admittedly, caching inside produces prettier interface with less clogs.
2. How to identify a certain capability.
    0. Should we separate pointer event listener and gesture recognizers as separate capabilities?
        1. No we shouldn't. 
            1. ~~Pointer event listeners, as long as they are hit-tested in the newest frame, will always receive hit positions with *up-to-date* transform.~~ (False, flutter's pointer event also uses snapshotted transform) While gesture recognizers, takes a snapshot.
            2. Essentially, push to gesture recognizer and push to pointer event listeners should be two disparate channel. No one wants interference from pointer event listener while performing gesture recognition. Therefore, there need to be at least two handler methods.
    1. Interface query table
        1. For description, see `epgi-core::foundation::query_interface`
        2. Disadvantage: user can misimplement table since by casting into wrong interface different from the table key.
        3. Disadvantage: very heavy-weight to implement
        4. Advantage: the table is **static** and **declarative**. We can query the capability **before** having a hit position.
            1. This means we can filter hit-test candidates much faster without computing any transform.
    2. Type-erased any-in-any-out method.
        1. Disadvantage: too much boilerplate involving Any. Not type-safe.
    3. Decision: interface query table. The advantage is too crucial.
3. Incompatible double polymorphism (\*coined term) (Serve as a note to justify seemingly verbose interface design)
    1. From the hit-tester, we want to be polymorphic over render object types. From the render object, we want to be polymorphic over screen canvas type. However, their communication depends on hit position transfomations, which can not satisfy both polymorphism, mainly due to varying target canvas type.
    2. This incompatibility, if exist, indicates there would be at least two dynamic dispatch on either side of call flow instead of one (excluding the one possible dynamic dispatch from the initiator) 
        1. Our impl introduces an extra dynamic dispatch at `ChildRenderObjectWithCanvas<C>` level.

Correction: The capability is not only generic over hit position, but it is also generic over `Protocol::Transform` as well if we chose not to cache it during painting!!!
1. If we continue with the transparent temporary tuple, it means an additional tuple element!!!!! (Adopted)








# Miscellaneous Flutter notes
1. Single tap never declares victory
2. Double tap can be declared victory before the first tap even finishes
3. What happens with the second pointer down event in a double tap, when the arean is held? Won't it add recognizers into a closed arena?
    1. No, Flutter's `PointerEvent::pointer` is not a pointer_id, it is a unique pointer interaction id that is not re-used.
        1. Therefore, the second pointer down creates a different arena than the first one. The two arenas coexist.
    1. DoubleTapGestureRecognizer::addAllowedPointer. Always tracks tap and register itself into arena.
    2. The process when the second pointer down event is fired
        1. GestureBinding::dispatchEvent dispatching new pointer event with new pointer id to the hit test result
        2. GestureBinding::dispatchEvent dispatch to itself and calls pointerRouter and route to previous subscribers
        2. pointerRouter invokes DoubleTapGestureRecognizer::_handleEvent -> 
        3. DoubleTapGestureRecognzier::_registerSecondTap ->
        4. claims victory in arena and DoubleTapGestureRecognizer::_reset ->
        5. GestureArena::release ->
        6. GestureArena::sweep ->
        7. removes the current arena and resolves in favor of double tap
4. How does single tap handles multi touch scenario?
    1. PrimaryPointerGestureRecognizer still participates in each arena, but
    2. It will only handle events from the pointer that fired the down event when it was in ready state (primaryPointer)
        1. PrimaryPointerGestureRecognizer::addAllowedPointer: state: ready -> possible
        2. OneSequenceGestureRecognizer::stopTrackingPointer(if empty tracked pointer) -> PrimaryPointerGestureRecognizer::didStopTrackingLastPointer: state: * -> ready
    3. BaseTapGestureRecognizer will also only invoke callbacks on winning the primaryPointer arena.
5. What is the purpose of `PointerAddEvent`/`PointerRemoveEvent`/`PointerEnterEvent`/`PointerExitEvent`
    1. `PointerEnterEvent`/`PointerExitEvent` are synthetic events generated by `RenderMouseRegion` which is invoked from `MouseTrackerAnnotation`
    2. `MouseTrackerAnnotation::onEnter` is invoked from `MouseTracker::_handleDeviceUpdateMouseEvents`
    3. The annotation is registered in `MouseTracker::_hitTestInViewResultToAnnotations` and compare to old caches to calculate the diff.
    4. `PointerAddEvent`/`PointerRemoveEvent` are pure marker event in these processes with no other uses. 
6. PointerSignalEvent
    1. PointerSignalEvent need a PointerSignalResolver https://api.flutter.dev/flutter/gestures/PointerSignalResolver-class.html
    2. Decision: on hold
7. Scroll amount
    1. Experiment: On gnome wayland, 1 tick = 53px
    2. https://github.com/flutter/engine/pull/32094
8. How does flutter keep paint transform and hit test transform? (In theory, the two should be inverse to each other and one may simply not exist)
    1. `BoxHitTestResult::addWithPaintTransform` called `Matrix4:::tryInvert`
9. Most of flutter's widget actually uses `HitTestBehavior::deferToChild` semantics, which makes them not claiming the  hit test by default
    1. This even includes default PointerListener and RawGestureDetectorState
    2. Then how do pointer listener and raw gesture detector enroll in hit test?
        1. RenderColorBox uses opaque semantics.
        2. A lot of high-level widgets uses `GestureDetector` in an opaque manner.
        3. We should follow their approach. Separating capability query and hit-test concrete-ness.
            1. And do not cut tree-walks based on concreteness. This will allow us to unify hit-testing in 3D.
                1. The tree-walk cuts in theory should be best handled by a render object, not a scheduler extension. Because 2D and 3D would have vastly different behavior, and they could be nested
                    1. This creates the problem of where to fit the hit test inside the pipeline
                        1. Handled in render object callback, initiated by a simplistic gesture bindings in scheduler extension
                    2. This enable us to make capabilities non-generic!!! Excellent news!
                        1. Now 2D PointerManager only collects 2D PointerHandler. 3D works in a similar manner!!! They only delivers event to their respect scope, and do not probe too deep into descendent.
                        2. Now the gesture recognition system becomes fractal instead of centralized!!!!



# Sources of updates
1. Pointer events
2. Recognizer timer
3. Recognizer reconciliation changes during rebuild (Could cause the recognizer to be gone for good, i.e. loss of stable identity)
4. Updates from the same gesture recognizer from an associated arena update.


# Notes during the new design
1. Gesture recognizer: shared instance across all arena, or create unique instance for each new arena?
    1. If we want create unique instances, then we need a way to clean-up retired references inside render object.
    2. The advantage is that we may create fully lock-free gesture recognizers, say tap recognizer.
    3. Decision: Shared instance, as we can follow Flutter's logic. And more complex gesture recognizers will almost certainly need locks.