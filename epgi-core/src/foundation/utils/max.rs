#[macro_export]
macro_rules! max {
    ($x:expr) => ( $x );
    ($x:expr, $($xs:expr),* $(,)?) => {
        std::cmp::max($x, max!( $($xs),+ ))
    };
}