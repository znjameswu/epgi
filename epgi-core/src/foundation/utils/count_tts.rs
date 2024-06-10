// // We save these comment to demonstrate how to impl similary functionalities in other macros

// #[macro_export]
// macro_rules! replace_expr {
//     ($_t:tt $sub:expr) => {$sub};
// }

// #[macro_export]
// macro_rules! count_tts {
//     ($($tts:tt)*) => {<[()]>::len(&[$(replace_expr!($tts ())),*])};
// }
