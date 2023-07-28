// use crate::foundation::{SyncMutex, Arc, Aweak};

// use super::Affine2d;

// pub struct LayerGroup {
//     inner: SyncMutex<Vec<Arc<Layer>>>,
// }

// struct LayerGroupInner {
//     layers: Vec<Arc<Layer>>
// }


// pub struct Layer {
//     group: Aweak<LayerGroup>,
//     inner: SyncMutex<LayerInner>
// }

// struct LayerInner {
//     old_fragment: vello::SceneFragment,
//     new_fragment: vello::SceneFragment,
//     /// The transformation from scene
//     transform: Affine2d
// }

