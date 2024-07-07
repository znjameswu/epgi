extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Index, ItemStruct};

#[proc_macro_derive(Declarative)]
pub fn derive_declarative(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);

    let struct_name = input.ident;

    let _struct_name = format_ident!("_{}", struct_name);
    // The following macro is nothing but a quirk of miracle
    // If you don't `pub use` the macro, then you get https://github.com/rust-lang/rust/pull/52234#issuecomment-786557648.
    // Basically, you won't be able to use the macro inside its origin crate except for direct sibling module (not even inside cousin module or uncle modules)
    // If you do `pub use` but not rename, you get error.
    // If you do `pub use`, rename to _struct_name, but also rename inside the expanded body to _struct_name, you get macro expand failure.
    // However, if you `pub use`, rename at `pub use` and `macro_rules` only, use unrenamed macro names inside the macro body, the whole crap just works.
    quote! {
        #[macro_export]
        macro_rules! #_struct_name {
            ($($key:ident $(= $value:expr)? ), * $(,)?) => {
                {
                    let builder = #struct_name::builder();
                    $(#struct_name!(@setter_helper builder $key $($value)?);)*
                    builder.build()
                }
            };
            (@setter_helper $builder:ident $key:ident $value:expr) => {
                let $builder = $builder.$key($value);
            };
            (@setter_helper $builder:ident $key:ident) => {
                let $builder = $builder.$key($key);
            };
        }

        pub use #_struct_name as #struct_name;
    }
    .into()
}

#[proc_macro_derive(Lerp)]
pub fn derive_lerp(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let struct_name = input.ident;
    let fields = match &input.fields {
        syn::Fields::Named(fields) => fields
            .named
            .iter()
            .map(|field| {
                let ident = field.ident.as_ref().expect("All fields must be named in the struct");
                quote! {
                    #ident: self.#ident.lerp(&other.#ident, t)
                }
            })
            .collect(),
        syn::Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, _field)| {
                let ident = Index::from(i);
                quote! {
                    #ident: self.#ident.lerp(&other.#ident, t)
                }
            })
            .collect(),
        syn::Fields::Unit => Vec::new(),
    };
    quote! {
        impl Lerp for #struct_name {
            fn lerp(&self, other: &Self, t: f32) -> Self {
                Self {
                    #(#fields),*
                }
            }
        }
    }
    .into()
}
