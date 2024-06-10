#[macro_export]
macro_rules! read_providers {
    ($values: expr, $($t: tt),* $(,)?) => {
        {
            const N: usize = {read_providers!(@count_tts $($t )*)};
            #[allow(non_snake_case)]
            let [$($t),*] = $values.try_into_array::<N>()
                .map_err(|values| values.len())
                .expect(&std::format!("Error reading provider values. Expected {} providers. Found count: ", N));
            (
                $(
                    $t.downcast::<$t>()
                        .map_err(|value| value.type_name())
                        .expect(&std::format!("Error reading provider values. Expected provider of type: {}. Found type: ", std::any::type_name::<$t>()))
                ),*
            )
        }
    };
    (@count_tts $($tts: tt)*) => {
        [
            $(read_providers!(@replace_expr $tts ())),*
        ].len()
    };
    (@replace_expr $_t:tt $sub:expr) => {$sub};
}

#[macro_export]
macro_rules! read_one_provider_into {
    ($x: ident, $values: ident, $t: ty) => {
        let mut $values = $values.into_iter();
        let $x = $values
            .next()
            .expect("Error reading provider values. Tried to read one provider when there are none left.")
            .downcast::<$t>()
            .map_err(|value| value.type_name())
            .expect(&std::format!("Error reading provider values. Expected provider of type: {}. Found type: ", std::any::type_name::<$t>()));

    };
}
