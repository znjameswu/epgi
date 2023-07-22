


# Layout protocol concepts
## Overview of the layout process

The layout process inherently involves the interaction between elements, and produces a state-depending layout result. The process can be summarized as
```
RenderObjectLayoutLogic(LayoutProtocol).call(RenderObjectStates, ChildLayoutResults) = LayoutResult
```
Layout protocol is merely the communication process between the parent and the child render object with the purposes:
1. To restrict the uncountable number of possible layout outcomes to some subspaces with some commonly-used degrees of freedom. For example, in a box layout protocol, we restrict children to be axis-aligned rectangules while they can be freely moved around and resized. In an arc layout protocol, we restrict children to be isocenter arcs while they can be freely rotated along the origin or resized in angular span or radius. The parent render object layout logic can then more easily express the mapping from the states to the layout results with the reduced degrees of freedom in the output subspace.
2. To express the request from parent to the child as a set of constraints (Constraints).
3. To express the request from child to the parent as a set of invariants (Span).






## Protocol explained using plain geometric objects
Each laid-out object has its own *reference point* and *reference orientaion*. The recommended *reference point* is to set the reference point at one specific "starting" corner of its span. (E.g. box-shape object at its top-left corner). 

A reference point + a reference orientation + coordinate scheme = a *coordinate system*! A coordinate system uniquely maps *coordinate*s to canvas points. The layout process determines the *coordinate* of the child's *reference point* and the *reference orientation* in its parent's *coordinate system*.

A *span* describes how much space a child object takes up in this coordinate space. For example, an axis-aligned rectangle can have a span of `(dx, dy)` no matter where it is placed.

When a protocol is specified, the parent gives children *constraints* and children return their *span*, then the parent gives the *coordinate* for each child.


## Generic protocol explanation
1. *Span* represents the invariant of a laid out child. Which means despite all the degrees of freedom the parent has over the children, those specified by *Span* has been given to the children and once the children has decided the parent must ensure to satisfy those invariants in all possible layout.
2. *Coordinate* represents all the possible degrees of freedom the parent has control over in the layout result. 
3. Each render object has a *reference* in its coordinate system for the layout of its content (mainly its children). It has no effect when viewed from within the render object itself (Children generated with a fixed coordinate in a coordinate system always have a fixed coordinate in that coordinate system :) ), and only has meaning when interacting with external coordinate systems. The coordinate of a child in the parent's coordinate system corresponds the *reference* for the child.
4. *Canvas reference* is what the render object's reference actually look like on the current painting canvas's coordinate system. Canvas always uses a cartesian system and serves as an absolute reference of everthing painted on the canvas. Note, the canvas coordinate system is still not the on-screen coordinate system. A new canvas can be pushed and take transformations if the render object requests.
5. *Coordinate scheme* is how you construct a coordinate mapping relations from a reference. Different coordinate scheme is implied by different protocols.
6. *Constraints* delivers runtime layout context information from the parent to the children. It imposes extra loose constraints on the child layout results. It is the only information source the child layout has on the outside world. Children *may* choose to break the constraints. Parents *may* choose to display warning messages if a child breaks constraints.


### How the painting works
When painted, the render object will be given its coordinate given by the parent, its parent's canvas reference, and its span returned to the parent. The render object should wholy determine how it should be painted based these three information. (Why its parents' canvas reference? Because we have to talk under one same coordinate system! Do not mixing up coordinate systems from different protocols!)
`paint(parent_canvas_reference: SP::CanvasReference, self_pos: SP::Coordinate, self_span: SP::Span)`

Render objects are also responsible to calculate the canvas reference for itself `compute_canvas_reference(parent_canvas_reference: SP::CanvasReference, self_pos: SP::Coordinate) -> CP::CanvasReference`

It is recommended to avoid passing `CanvasReference` with non-zero rotations. Instead, you should push a new canvas with the transformations in the `paint` call, and then return a default canvas reference when asked to compute. Pushing a fresh canvas with default canvas reference helps to speed up painting phase by introducint a new independent paint root to start from.

#### Matrix3 general CanvasReference
`compute_child_transform(parent_canvas_reference: CanvasReference, child_pos: CP::Coordinate) -> CanvasReference`
This function is vectorizable.

This creates problem for 3D renderer.

### How those states are stored in tree
```
      Node A
     ┌──────────────────────┐
     │ SelfProtocol         │
     ├──────────────────────┤
     │ Received Constraints │
     │ Previous Span        │
     │ Recieved Coordinates │
     │ Cached Intrinsics    │
     │ Paint Logic          │
     ├──────────────────────┤
     │ ChildProtocol        │
     ├──────────────────────┤
┌────┤ Canvas Reference     │
│    │                      │
│ ┌──┤ Layout logic         │◄─┐
│ │  └──────────────────────┘  │
│ │                            │
│ │   Node B                   │
│ │  ┌──────────────────────┐  │
│ │  │ SelfProtocol         │  │
│ │  ├──────────────────────┤  │
│ ├─►│ Received Constraints │  │
│ │  │ Previous Span        ├──┘
│ └─►│ Recieved Coordinates │
│    │ Cached Intrinsics    │
├───►│ Paint Logic          │
│    ├──────────────────────┤
│    │ ChildProtocol        │
│    ├──────────────────────┤
└───►│ Canvas Reference     │
     │                      │
     │ Layout logic         │
     └──────────────────────┘
```
# Examples of layout protocol
## RingChart with curved text
This would require the following widget levels
1. `RingChart` with `SP=BoxProtocol, CP=RingChartProtocol`
2. `Ring` with `SP=RingChartProtocol, CP=RingProtocol`
3. `Adapter<RingProtocol, IsometricCurveProtocol>`
4. `CurvedText` with `SP=IsometricCurveProtocol`
    1. Internally, `Adapter<IsometricCurveProtocol, SingleLineProtocol>`
    2. `RawGrapheme` with `SP=SingleLineProtocol`

A proposed implementation of those protocols would be
1. `RingChartProtocol`
    1. `type Span = {angular: f32, radial: f32}`
    2. `type Coordinate = {angular: f32, radial: f32}`
    3. `type Constraints = {max..., min...}`,
    4. `type CanvasReference = {x: f32, y: f32, angle:f32}` points to the canvas position of the center of the rings.
2. `RingProtocol`
    1. `type Span = {angular: f32, radial: f32}`
    2. `type Coordinate = {angular: f32, radial: f32}`
    3. `type Constraints = {max..., min...}`
    4. `type CanvasReference = {x: f32, y: f32, angle:f32, radius: f32}` where `(x,y)` is the canvas position of the center of the rings, `(angle, radius)` is where this ring locates in the ring graph.
3. `IsometricCurveProtocol`
    1. `type Span = f32`
    2. `type Coordinate = f32`
    3. `type Constraints = {max..., min...}`
    4. `type CanvasReference = {x: f32, y: f32, angle:f32, curve: Asc<dyn Fn(f32) -> (f32,f32)>}` where `(x,y)` is the center of the starting edge of the ring, `angle` is starting tangential direction of the ring, `curve` is the isometric parameteric curve.
4. `SingleLineProtocol` is just `BoxProtocol` with a baseline.

## A naive and greedy multi-line text layout protocol, e.g. for math equation rendering
1. `type Intrinsics = MinFirstRunWidth | RunCount | CanBreakAtEnd`
1. `type Constraints = {next_run_width: f32, max_first_line_width: f32, full_line_width: f32}`,
2. `type Span = {line_spans: Vec<{width: f32, ascent: f32, descent: f32}>}`
3. `type Coordinate = {line_coord: Vec<BoxCoordinate>}`
4. `type CanvasReference = Vec<{x: f32, y: f32, angle:f32}>` points to the canvas position of the center of the rings.




# A simpler relayout boundary mechanism
In flutter, a relayout boundary is determined by multiple factors, such as:
1. 

However, a lot of them is not necessary if we enforce a explicit parent_use_size