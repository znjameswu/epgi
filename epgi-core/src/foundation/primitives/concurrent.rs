// // Reference data: https://github.com/xacrimon/conc-map-bench
// pub type ConcurrentMap<K, V, S> = dashmap::DashMap<K, V, S>;

/**************************** Concurrent channels *******************************/
// https://github.com/fereidani/rust-channel-benchmarks

// pub type AsyncMpscSender<T> = async_channel::Sender<T>;
// pub type AsyncMpscReceiver<T> = async_channel::Receiver<T>;
// pub fn unbounded_channel_async<T>() -> (AsyncMpscSender<T>, AsyncMpscReceiver<T>) {
//     async_channel::unbounded()
// }

// pub fn bounded_channel_async<T>(cap: usize) -> (AsyncMpscSender<T>, AsyncMpscReceiver<T>) {
//     async_channel::bounded(cap)
// }

pub type SyncMpscSender<T> = crossbeam::channel::Sender<T>; // Try flume?
pub type SyncMpscReceiver<T> = crossbeam::channel::Receiver<T>;
pub fn unbounded_channel_sync<T>() -> (SyncMpscSender<T>, SyncMpscReceiver<T>) {
    crossbeam::channel::unbounded()
}

pub fn bounded_channel_sync<T>(cap: usize) -> (SyncMpscSender<T>, SyncMpscReceiver<T>) {
    crossbeam::channel::bounded(cap)
}

pub type MpscQueue<T> = crossbeam::queue::SegQueue<T>;
