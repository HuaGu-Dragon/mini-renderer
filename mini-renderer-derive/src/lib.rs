use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Varying)]
pub fn derive_varying(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;
    let syn::Data::Struct(data) = input.data else {
        panic!("Only support struce yet")
    };

    let fields = data.fields;

    let varying = fields.iter().map(|f| {
        let ident = &f.ident;
        quote! {
            #ident: ::mini_renderer::pipeline::varying::Varying::interpolate(v0.#ident, v1.#ident, v2.#ident, w0, w1, w2)
        }
    });

    quote! {
        impl ::mini_renderer::pipeline::varying::Varying for #ident {
            fn interpolate(v0: Self, v1: Self, v2: Self, w0: f32, w1: f32, w2: f32) -> Self {
                Self {
                    #(#varying),*
                }
            }
        }
    }
    .into()
}
