use epgi_core::tree::{ArcAnyRenderObject, AweakAnyRenderObject, Render};

use super::{
    PointerActiveData, PointerBaseData, PointerEvent, PointerHoverData, PointerPanZoonUpdateData,
    PointerSignalData,
};

pub trait PointerEventHandler {
    fn handle_pointer_event(&self, event: PointerEvent);
}

/// GestureHandler differs from PointerEventHandler in that gesture recognition is arena-based. 
/// Once a winner is resolved, all other competitors lose and will no longer receives event.
pub trait GestureRecognizer {
    /// Return: Confidence.
    /// Confidence <=0.0 equals to declaring defeat and immediate withdrawal.
    /// Confidence >= 1.0 equals to declaring victory and requesting immediate resolution.
    /// 
    /// Declaring a victory does not mean a recognition will be granted. There could be another regonizer also declaring victory with a higher confidence. The highest one will win.
    /// Declaring a defeat, however, does mean a withdrawal. If everyone withdraws, the arena will recognize no victor.
    fn compete_pointer_event(&self, event: PointerEvent) -> f32;

    fn handle_arena_victory(&self);
}

pub trait PointerEventHandlerObject {
    fn handle_pointer_hover(
        &self,
        base: PointerBaseData,
        hover: PointerHoverData,
        synthesized: bool,
    );
    fn handle_pointer_down(&self, base: PointerBaseData, active: PointerActiveData);
    fn handle_pointer_move(
        &self,
        base: PointerBaseData,
        active: PointerActiveData,
        synthesized: bool,
    );
    fn handle_pointer_up(&self, base: PointerBaseData, hover: PointerHoverData);
    fn handle_pointer_cancel(&self, base: PointerBaseData);
    fn handle_pointer_add(&self, base: PointerBaseData);
    fn handle_pointer_remove(&self, base: PointerBaseData);
    fn handle_pointer_pan_zoom_start(&self, base: PointerBaseData, synthesized: bool);
    fn handle_pointer_pan_zoom_update(
        &self,
        base: PointerBaseData,
        update: PointerPanZoonUpdateData,
        synthesized: bool,
    );
    fn handle_pointer_pan_zoom_end(&self, base: PointerBaseData, synthesized: bool);
    fn handle_pointer_signal(&self, base: PointerBaseData, signal: PointerSignalData);
}
