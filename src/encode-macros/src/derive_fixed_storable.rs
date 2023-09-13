use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse, Data, DataStruct, DeriveInput, Field, Fields, Generics, Ident, Index, Type};

pub fn derive_fixed_storable(tokens: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse(tokens).unwrap();

    match ast.data {
        Data::Struct(struct_data) => {
            derive_fixed_storable_struct(&struct_data, &ast.ident, &ast.generics)
        }
        Data::Enum(_) => panic!("enums not supported"),
        Data::Union(_) => panic!("unions not supported"),
    }
}

fn derive_fixed_storable_struct(
    struct_data: &DataStruct,
    struct_name: &Ident,
    generics: &Generics,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let max_size_tokens = generate_max_size(struct_data);
    let is_fixed_tokens = generate_is_fixed(struct_data);

    let field_names = get_field_names(struct_data);
    let field_types = get_field_types(struct_data);

    let serialize_field_tokens = quote::quote! {
        #(
            result.extend_from_slice(&self.#field_names.to_bytes());
        )*
    };

    // use universal names that wotk both for named fields and in a tuple
    let local_names: Vec<_> = field_names
        .iter()
        .map(|id| quote::format_ident!("decoded_{}", id.to_string()))
        .collect();
    let constructor_tokens = match struct_data.fields {
        Fields::Named(_) => quote::quote! {
            Self {
                #(
                    #field_names: #local_names,
                )*
            }
        },
        Fields::Unnamed(_) => quote::quote!(Self(#(#local_names, )*)),
        Fields::Unit => quote::quote!(Self()),
    };

    quote::quote!(
        impl #impl_generics ic_stable_structures::Storable for #struct_name #ty_generics #where_clause {
            fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
                let full_size = #max_size_tokens;

                let mut result = std::vec::Vec::with_capacity(full_size as _);
                #serialize_field_tokens

                result.into()
            }

            fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
                let mut offset: usize = 0;

                fn get_size<T: BoundedStorable>(_: &T) -> u32 {
                    T::MAX_SIZE
                }

                #(
                    let size = <#field_types as BoundedStorable>::MAX_SIZE as usize;
                    let #local_names: #field_types = Storable::from_bytes((&bytes.as_ref()[offset..(offset + size)]).into());
                    offset += size;
                )*

                #constructor_tokens
            }
        }

        impl #impl_generics ic_stable_structures::BoundedStorable for #struct_name #ty_generics #where_clause {
            const MAX_SIZE: u32 = #max_size_tokens;

            const IS_FIXED_SIZE: bool = {
                let is_fixed = #is_fixed_tokens;
                assert!(is_fixed);

                is_fixed
            };
        }
    ).into()
}

fn get_fields(struct_data: &DataStruct) -> Vec<&'_ Field> {
    match &struct_data.fields {
        Fields::Named(fields) => fields.named.iter().collect(),
        Fields::Unnamed(fields) => fields.unnamed.iter().collect(),
        Fields::Unit => Vec::new(),
    }
}

fn get_field_types(struct_data: &DataStruct) -> Vec<&'_ Type> {
    get_fields(struct_data).iter().map(|f| &f.ty).collect()
}

fn get_field_names(struct_data: &DataStruct) -> Vec<impl ToTokens + ToString> {
    get_fields(struct_data)
        .iter()
        .enumerate()
        .map(|(index, field)| match &field.ident {
            Some(name) => quote::quote!(#name),
            None => {
                let index = Index::from(index);
                quote::quote!(#index)
            }
        })
        .collect()
}

fn generate_max_size(struct_data: &DataStruct) -> impl ToTokens {
    let struct_types = get_field_types(struct_data);
    quote::quote! { #(<#struct_types as BoundedStorable>::MAX_SIZE)+* }
}

fn generate_is_fixed(struct_data: &DataStruct) -> impl ToTokens {
    let struct_types = get_field_types(struct_data);
    quote::quote! { #(<#struct_types as BoundedStorable>::IS_FIXED_SIZE) &&* }
}
