# Faithful Interactivity
Definition: user input will only be handled according to what was already rendered on the screen.
Motivation: Normal build process will handle the event based on the *current* layout tree, while the rendered content on the screen may be one or two frames outd.
Design: We have to keep two RenderObject trees. One is for `released`, and another one for `stashed`, (and possible another `head`). We process event according to the `released` and work on `head`. Everytime layout is finished, `head` forks a clone into `stashed`. Everytime vsync is trigger, `stashed` overwrites `released`.


Double buffering
```
VSYNC                                                VSYNC
  │                                                    │
──┤   ┌───────┬────────┬───────┬───────────┐           │
  │   │       │        │       │           │           │
──┴─┐ │       │        │       │           │           │
    │ │ Build │ Layout │ Paint │Composition│           │
────┴─┤       │        │       │           │           │
      │       │        │       │           │           │
──┬─┬─┴───────┴────────┴───────┴───────────┘           │
  │ │                                                  │
  ├─┴──────────────────────────────────────────────────┤   ┌───────┬────────┬───────┬───────────┐
  │                 Event Collection                   │   │       │        │       │           │
  ├─┬──────────────────────────────────────────────────┴─┐ │       │        │       │           │
  │ │            Feedback Event Dispatch                 │ │ Build │ Layout │ Paint │Composition│
  │ ├──────────────────┬─────────────────────────────────┴─┤       │        │       │           │
  │ │                  │    Oneway Event Dispatch          │       │        │       │           │
  │ │                  └───────────────────────────────┬─┬─┴───────┴────────┴───────┴───────────┘
  │ │Swap Renderobject Tree                            │ │
  │                                                    ├─┴────────────────────────────────────────
  │                                                    │          Event Collection
  │                                                    ├─┬────────────────────────────────────────
  │                                                    │ │            Feedback Event Dispatch
  │                                                    │ ├──────────────────┬─────────────────────
  │                                                    │ │                  │   Oneway Event Disp
  │                                                    │ │                  └─────────────────────
  │                                                    │ │Swap Renderobject Tree
```

Triple buffering
```
          VSYNC                          *: Oneway Event Dispatch                       VSYNC
            │                                                                             │
┌───────┬───┴────┬───────┬───────────┐                                                    │
│       │        │       │           │                                                    │
│ Build │ Layout │ Paint │Composition│                                                    │
│       │        │       │           │                                                    │
└───────┴───┬────┴───────┴───────────┘                                                    │
            │                                                                             │
────────────┤    ┌───────┬───────┬────────┬───────┬───────────┐                           │
Event Collec│    │   *   │       │        │       │           │                           │
────────────┴───┬┴───────┤ Build │ Layout │ Paint │Composition│                           │
Feedback Ev Disp│        │       │        │       │           │                           │
────────────┬───┤        └───────┴────────┴───────┴───────────┘                           │
            │   │                                                                         │
            ├───┴─────────────────────────┬───────┬───────┬────────┬───────┬───────────┐  │
            │      Event Collection       │   *   │       │        │       │           │  │
            ├───┬─────────────────────────┴───────┤ Build │ Layout │ Paint │Composition│  │
            │   │     Feedback Event Dispatch     │       │        │       │           │  │
            │   ├─────────────────────────────────┴───────┴────────┴───────┴───────────┘  │
            │   │                                                                         │
            │   │                         ┌────────────────────────┬───────┬───────┬──────┴─┬───────
            │   │                         │  Event Collection      │   *   │       │        │
            │   │                         ├────────────────────────┴───────┤ Build │ Layout │ Paint
            │   │                         │  Feedback Event Dispatch       │       │        │
            │   │                         └────────────────────────────────┴───────┴──────┬─┴───────
            │   │                                                                         │
            │   │                                                  ┌──────────────────────┤ ┌───────
            │   │                                                  │  Event Collection    │ │
            │Swap RenderObject Tree                                ├──────────────────────┴─┴─┬─────
            │                                                      │  Feedback Event Dispatch │
            │                                                      └──────────────────────┬───┤
            │                                                                             │   │
            │                                                                             ├───┴─────
            │                                                                             │Ev Disp
            │                                                                             ├───┬─────
            │                                                                             │   │F.E.D.
            │                                                                             │   ├─────
            │                                                                             │   │Swap
            │                                                                             │    RenderObject
            │                                                                             │    Tree
```
## Is there a reliable way to get frame timing (frame presentation complete event)?
No
1. https://github.com/KhronosGroup/Vulkan-Docs/issues/1158
2. https://github.com/KhronosGroup/Vulkan-Docs/issues/370
3. https://www.reddit.com/r/vulkan/comments/9ibcy3/synchronizing_vkqueuepresentkhr_from_multiple/

## What is the timing constraints under this mode?
Mode-independent Timing Constraints:
1. Feedback event dispatch must happen after event collection termination
2. Feedback event dispatch must happen after render object tree swap event if previous event collection termination was accompanied by the render object tree swap event.
3. Oneway event dispatch must happen after event collection termination.
4. Oneway event dispatch must happen after previous frame's layout phase.
5. Build phase must happen after both event dispatching completed of this frame.
6. Build phase must happen after previous frame's painting phase.
7. At the end of layout phase, there will be a fork off from `current` RenderObject Tree into `stashed` RenderObject tree. (Render object tree fork event).

If we can reliably set the swap chain size to be 2, then we could still infer presentation event by `get_current_texture`. However, `WebGPU` has no intention to expose swap chain size. `wgpu` hides the capability under `wgpu-hal::SurfaceConfiguration` and default it close to `wgpu-core::present::DESIRED_NUM_FRAMES=3` but still direct the call to `Vulkan`. But `Vulkan` simply allows implementation to smuggle more images into the swap chain without telling the user. So, no, this does not work.

# Multiple GPUQueue Rendering / Why a single raster thread instead of concurrent frame rasterization on GPU?
It should be theoretically possible to create multiple queues in Vulkan/DX12 and manage them in a multi-threaded CPU runtime to work on different frames concurrently. However, according to above threads, first of all, AMD only provide one graphics queue on Vulkan and time-slicing direct queues on DX12, making such effort impossible or non-profitable at best on AMD hardware. Secondly, WebGPU has completely [dropped this idea](https://github.com/gpuweb/gpuweb/issues/1065) in V1, specifying one GPUQueue for each GPUDevice and one GPUDevice for one GPUCanvasContextConfiguration, and even went as far as [implicit `present` command](https://github.com/gpuweb/gpuweb/issues/182). Even in post-V1 WebGPU they had only discussed one "main" queue and several compute queues. `wgpu` has an explicit `present` command but comply to other WebGPU spec on this regard.

Besides, restricting one rasterization at the same time greatly reduces complexity in faithful interactivity design. If there can be multiple frames in rasterization, then we have to track the layout information for each of those frame.

# Async batch commit at any time
Currently the async batch commits are only allowed to happen at specific time points via polling, particularly, at the start and the end of the synchronous build phase. If we allow them to freely commit upon completion, it would take up less budget of frame building time. 

However, it is detrimental to the pipeline design. Batch commit invalidates the element tree and render object tree immediately (if we do not implement faithful interactivity), disabling event dispatching (since the hit test dependes on valid layout information), forcing the event dispatching to end before the first async batch completion it encounters (which is unpredictable), and the event collection to stop even before that. The event distribution system would be in a very sorry and unpredictable shape. Hence we have to force async batch commits to happen at specific time points with regular intervals.

# PaintCommand for Non-layer node paint caching
`Canvas::PaintCommand` was an effort to enable caching of painting results for arbitrary render objects. It introduces another encoding scheme which would be more ergonomic and storable on top of vello encoding.

It was ultimately decided that caching would only be performed on layer and fragments, not for individual render objects.

However, `PaintCommand` would still function as a universal interface between epgi canvas and renderer implementations for various canvas protocols.