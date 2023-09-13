use proc_macro::TokenStream;

mod derive_fixed_storable;

/// Generates implementation of `Storable` and `BoundedStorable` traits
/// for the structure that contains only members that are `BoundedStorable` with
/// IS_FIXED set to true.
#[proc_macro_derive(FixedStorable)]
pub fn derive_fixed_storable(tokens: TokenStream) -> TokenStream {
    derive_fixed_storable::derive_fixed_storable(tokens)
}
