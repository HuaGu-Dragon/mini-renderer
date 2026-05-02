use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(Varying)]
pub fn derive_varying(input: TokenStream) -> TokenStream {
    quote! {}.into()
}
