extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

#[proc_macro_derive(Declarative)]
pub fn derive_declarative(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);

    let struct_name = input.ident;

    quote! {
        #[macro_export]
        macro_rules! #struct_name {
            // ($($key:ident $(= $value:expr)? ), * $(,)?) => {
            //     {
            //         let builder = #struct_name::builder();
            //         $(#struct_name!(@setter_helper builder $key $($value)?);)*
            //         builder.build()
            //     }
            // };
            // (@setter_helper $builder:ident $key:ident $value:expr) => {
            //     let $builder = $builder.$key($value);
            // };
            // (@setter_helper $builder:ident $key:ident) => {
            //     let $builder = $builder.$key($key);
            // };

            ($($key:ident = $value:expr ), * $(,)?) => {
                {
                    let builder = #struct_name::builder();
                    $(let builder = builder.$key($value);)*
                    builder.build()
                }
            };
        }
    }
    .into()
}
