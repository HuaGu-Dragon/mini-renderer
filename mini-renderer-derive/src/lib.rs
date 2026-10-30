use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input, parse_quote};

#[proc_macro_derive(Varying, attributes(flat))]
pub fn derive_varying(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match &input.data {
        syn::Data::Struct(data) => {
            let ident = input.ident;
            let mut generics = input.generics;
            add_trait_bounds(&mut generics);
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
            let varying = data.fields.iter().enumerate().map(|(n,field)| {
                if field.attrs.iter().any(|attr| attr.path().is_ident("flat")) {
                    if let Some(name)= &field.ident {
                        quote! {
                            #name: v0.#name,
                        }
                    } else {
                        let n = syn::Index::from(n);
                        quote! {
                            #n: v0.#n,
                        }
                    }
                } else {
                    if let Some(name)= &field.ident {
                        quote! {
                            #name: ::mini_renderer::pipeline::varying::Varying::interpolate(v0.#name, v1.#name, v2.#name, w0, w1, w2)
                        }
                    } else {
                        let n = syn::Index::from(n);
                        quote! {
                            #n: ::mini_renderer::pipeline::varying::Varying::interpolate(v0.#n, v1.#n, v2.#n, w0, w1, w2)
                        }
                    }
                }
            });

            quote! {
                impl #impl_generics ::mini_renderer::pipeline::varying::Varying for #ident #ty_generics #where_clause {
                    fn interpolate(v0: Self, v1: Self, v2: Self, w0: f32, w1: f32, w2: f32) -> Self {
                        Self {
                            #(#varying),*
                        }
                    }
                }
            }
            .into()
        }
        syn::Data::Enum(data) => {
            syn::Error::new_spanned(data.enum_token, "Varying cannot be derived for enums")
                .to_compile_error()
                .into()
        }
        syn::Data::Union(data) => {
            syn::Error::new_spanned(data.union_token, "Varying cannot be derived for unions")
                .to_compile_error()
                .into()
        }
    }
}

fn add_trait_bounds(generics: &mut syn::Generics) {
    for param in &mut generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            type_param
                .bounds
                .push(parse_quote!(::mini_renderer::pipeline::varying::Varying));
        }
    }
}
