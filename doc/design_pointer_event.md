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
Flutter defaults to pointer capture for all pointer events

W3C requires explicit setPointerCapture execpt for direct manipulation elements, https://w3c.github.io/pointerevents/#dfn-implicit-pointer-capture