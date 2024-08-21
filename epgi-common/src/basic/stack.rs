use std::sync::atomic::AtomicBool;

use epgi_2d::{
    Affine2dCanvas, Affine2dPaintContextExt, ArcBoxRenderObject, ArcBoxWidget, BlendMode,
    BoxConstraints, BoxIntrinsics, BoxMultiChildElement, BoxMultiChildElementTemplate,
    BoxMultiChildHitTest, BoxMultiChildLayout, BoxMultiChildPaint, BoxMultiChildRender,
    BoxMultiChildRenderTemplate, BoxOffset, BoxProtocol, BoxSize,
};
use epgi_core::{
    foundation::{
        set_if_changed, Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide,
        ThreadPoolExt,
    },
    scheduler::get_current_scheduler,
    template::ImplByTemplate,
    tree::{BuildContext, ElementBase, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::Alignment;

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Stack>))]
pub struct Stack {
    #[builder(default=Alignment::TOP_LEFT)]
    pub alignment: Alignment,
    #[builder(default=StackFit::Loose)]
    pub fit: StackFit,
    //TODO: Clip behavior
    pub children: Vec<Positioned>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum StackFit {
    Loose,
    Expand,
    Passthrough,
}

#[derive(Clone, Debug, Declarative, TypedBuilder)]
pub struct Positioned {
    #[builder(default, setter(strip_option))]
    pub l: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub r: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub t: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub b: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub width: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub height: Option<f32>,
    pub child: ArcBoxWidget,
}

#[derive(PartialEq, Clone, Debug)]
pub struct PositionedConfig {
    l: Option<f32>,
    r: Option<f32>,
    t: Option<f32>,
    b: Option<f32>,
    width: Option<f32>,
    height: Option<f32>,
}

impl Positioned {
    pub fn is_positioned(&self) -> bool {
        self.l.is_some()
            || self.r.is_some()
            || self.t.is_some()
            || self.b.is_some()
            || self.width.is_some()
            || self.height.is_some()
    }
    fn get_config(&self) -> PositionedConfig {
        PositionedConfig {
            l: self.l,
            r: self.r,
            t: self.t,
            b: self.b,
            width: self.width,
            height: self.height,
        }
    }
}

impl PositionedConfig {
    pub fn is_positioned(&self) -> bool {
        self.l.is_some()
            || self.r.is_some()
            || self.t.is_some()
            || self.b.is_some()
            || self.width.is_some()
            || self.height.is_some()
    }
}

impl From<ArcBoxWidget> for Positioned {
    fn from(child: ArcBoxWidget) -> Self {
        Positioned!(child)
    }
}

impl Widget for Stack {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type Element = StackElement;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

#[derive(Clone)]
pub struct StackElement {}

impl ImplByTemplate for StackElement {
    type Template = BoxMultiChildElementTemplate<false>;
}

impl BoxMultiChildElement for StackElement {
    type ArcWidget = Asc<Stack>;
    type Render = RenderStack;

    fn get_child_widgets(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<Vec<ArcBoxWidget>, BuildSuspendedError> {
        Ok(widget
            .children
            .iter()
            .map(|positioned| positioned.child.clone())
            .collect())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {}
    }

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderStack {
            alignment: widget.alignment,
            fit: widget.fit,
            positioned_configs: widget.children.iter().map(Positioned::get_config).collect(),
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        [
            set_if_changed(&mut render.alignment, widget.alignment),
            set_if_changed(&mut render.fit, widget.fit),
            set_if_changed(
                &mut render.positioned_configs,
                widget.children.iter().map(Positioned::get_config).collect(),
            ),
        ]
        .iter()
        .any(|&changed| changed)
        .then_some(RenderAction::Relayout)
    }
}

pub struct RenderStack {
    pub alignment: Alignment,
    pub fit: StackFit,
    pub positioned_configs: Vec<PositionedConfig>,
    //TODO: Clip behavior
}

impl ImplByTemplate for RenderStack {
    type Template = BoxMultiChildRenderTemplate<false, false, false, false>;
}

impl BoxMultiChildRender for RenderStack {
    type LayoutMemo = (Vec<BoxOffset>, bool);

    fn compute_intrinsics(
        &mut self,
        children: &Vec<ArcBoxRenderObject>,
        intrinsics: &mut BoxIntrinsics,
    ) {
        unimplemented!()
    }
}

impl BoxMultiChildLayout for RenderStack {
    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        children: &Vec<ArcBoxRenderObject>,
    ) -> (BoxSize, Self::LayoutMemo) {
        use std::sync::atomic::Ordering::*;
        debug_assert_eq!(
            children.len(),
            self.positioned_configs.len(),
            "RenderStack should receive the same amount of children as its positioned config"
        );
        if children.is_empty() {
            let biggest = constraints.biggest();
            if biggest.is_finite() {
                return (biggest, (Vec::new(), false));
            } else {
                return (constraints.smallest(), (Vec::new(), false));
            }
        }
        let non_positioned_constraints = match self.fit {
            StackFit::Loose => constraints.loosen(),
            StackFit::Expand => {
                let BoxSize { width, height } = constraints.biggest();
                BoxConstraints::new_tight(width, height)
            }
            StackFit::Passthrough => constraints.clone(),
        };

        let mut child_sizes = std::iter::repeat(BoxSize::ZERO)
            .take(children.len())
            .collect::<Vec<_>>();

        fn layout_non_positioned_child(
            child: &ArcBoxRenderObject,
            positioned_config: &PositionedConfig,
            child_size: &mut BoxSize,
            non_positioned_constraints: &BoxConstraints,
        ) -> Option<BoxSize> {
            if positioned_config.is_positioned() {
                return None;
            }
            *child_size = child.layout_use_size(non_positioned_constraints);
            Some(*child_size)
        }

        let threadpool = &get_current_scheduler().sync_threadpool;
        let biggest_non_positioned_size = threadpool.par_zip3_ref_ref_mut_map_reduce_vec(
            children,
            &self.positioned_configs,
            &mut child_sizes,
            |child, positioned_config, child_size| {
                layout_non_positioned_child(
                    child,
                    positioned_config,
                    child_size,
                    &non_positioned_constraints,
                )
            },
            || None,
            |size1, size2| match (size1, size2) {
                (None, None) => None,
                (None, Some(_)) => size2,
                (Some(_), None) => size1,
                (Some(size1), Some(size2)) => Some(BoxSize {
                    width: size1.width.max(size2.width),
                    height: size1.height.max(size2.height),
                }),
            },
        );

        let size = biggest_non_positioned_size.unwrap_or_else(|| constraints.biggest());

        assert_eq!(size, constraints.constrain(size));
        assert!(size.is_finite());

        fn layout_all_child(
            child: &ArcBoxRenderObject,
            positioned_config: &PositionedConfig,
            child_size: &mut BoxSize,
            size: BoxSize,
            alignment: Alignment,
            has_visual_overflow: &AtomicBool,
        ) -> BoxOffset {
            if !positioned_config.is_positioned() {
                alignment.along_offset(BoxOffset {
                    x: size.width - child_size.width,
                    y: size.height - child_size.height,
                })
            } else {
                let mut child_constraints = BoxConstraints::default();
                let PositionedConfig {
                    l,
                    r,
                    t,
                    b,
                    width,
                    height,
                } = positioned_config;
                if let (Some(l), Some(r)) = (l, r) {
                    child_constraints = child_constraints.tighten_width(size.width - l - r);
                } else if let Some(width) = width {
                    child_constraints = child_constraints.tighten_width(*width)
                }

                if let (Some(t), Some(b)) = (t, b) {
                    child_constraints = child_constraints.tighten_height(size.height - t - b);
                } else if let Some(height) = height {
                    child_constraints = child_constraints.tighten_height(*height)
                }

                *child_size = child.layout_use_size(&child_constraints);

                let x = if let Some(l) = l {
                    *l
                } else if let Some(r) = r {
                    size.width - r - child_size.width
                } else {
                    alignment
                        .along_offset(BoxOffset {
                            x: size.width - child_size.width,
                            y: size.height - child_size.height,
                        })
                        .x
                };

                let y = if let Some(t) = t {
                    *t
                } else if let Some(b) = b {
                    size.height - b - child_size.height
                } else {
                    alignment
                        .along_offset(BoxOffset {
                            x: size.width - child_size.width,
                            y: size.height - child_size.height,
                        })
                        .y
                };

                if x < 0.0
                    || x + child_size.width > size.width
                    || y < 0.0
                    || y + child_size.height > size.height
                {
                    has_visual_overflow.store(true, Relaxed);
                }

                BoxOffset { x, y }
            }
        }

        let has_visual_overflow = AtomicBool::new(false);

        let offsets = threadpool.par_zip3_ref_ref_mut_map_collect_vec(
            children,
            &self.positioned_configs,
            &mut child_sizes,
            |child, positioned_config, child_size| {
                layout_all_child(
                    child,
                    positioned_config,
                    child_size,
                    size,
                    self.alignment,
                    &has_visual_overflow,
                )
            },
        );

        (size, (offsets, has_visual_overflow.load(Relaxed)))
    }
}

impl BoxMultiChildPaint for RenderStack {
    fn perform_paint(
        &self,
        &size: &BoxSize,
        &offset: &BoxOffset,
        memo: &Self::LayoutMemo,
        children: &Vec<ArcBoxRenderObject>,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        let (child_offsets, has_visual_overflow) = memo;
        debug_assert_eq!(children.len(), child_offsets.len());
        if !has_visual_overflow {
            for (&child_offset, child) in std::iter::zip(child_offsets, children) {
                paint_ctx.paint(child, &(offset + child_offset));
            }
        } else {
            // TODO: clip behavior
            paint_ctx.clip_rect(offset & size, BlendMode::default(), 1.0, |paint_ctx| {
                for (&child_offset, child) in std::iter::zip(child_offsets, children) {
                    paint_ctx.paint(child, &(offset + child_offset));
                }
            });
        };
    }
}

impl BoxMultiChildHitTest for RenderStack {}
