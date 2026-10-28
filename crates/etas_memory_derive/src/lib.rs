use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Index, parse_macro_input, parse_quote};

#[proc_macro_derive(RetainedSize)]
pub fn derive_retained_size(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let field_types: Vec<_> = match &input.data {
        Data::Struct(data) => data.fields.iter().map(|field| field.ty.clone()).collect(),
        Data::Enum(data) => data
            .variants
            .iter()
            .flat_map(|variant| variant.fields.iter().map(|field| field.ty.clone()))
            .collect(),
        Data::Union(_) => vec![],
    };
    let mut generics = input.generics.clone();
    for parameter in generics.type_params_mut() {
        parameter
            .bounds
            .push(parse_quote!(::etas_core::memory::RetainedSize));
    }
    let (implementation, arguments, bounds) = generics.split_for_impl();
    let mut suffix = 0;
    let lifetime = loop {
        let candidate: syn::Lifetime = syn::parse_str(&format!("'__etas_memory{suffix}")).unwrap();
        if !input
            .generics
            .lifetimes()
            .any(|parameter| parameter.lifetime == candidate)
        {
            break candidate;
        }
        suffix += 1;
    };
    let body = match input.data {
        Data::Struct(data) => {
            let members = data.fields.iter().enumerate().map(|(index, field)| {
                let member = field
                    .ident
                    .clone()
                    .map(syn::Member::Named)
                    .unwrap_or_else(|| syn::Member::Unnamed(Index::from(index)));
                quote!(visitor.enqueue(&self.#member);)
            });
            quote!(#(#members)*)
        }
        Data::Enum(data) => {
            let variants = data.variants.iter().map(|variant| {
                let variant_name = &variant.ident;
                let bindings: Vec<_> = (0..variant.fields.len())
                    .map(|index| format_ident!("__field_{index}"))
                    .collect();
                let pattern = match &variant.fields {
                    Fields::Named(fields) => {
                        let names = fields
                            .named
                            .iter()
                            .map(|field| field.ident.as_ref().unwrap());
                        quote!({ #(#names: #bindings),* })
                    }
                    Fields::Unnamed(_) => quote!(( #(#bindings),* )),
                    Fields::Unit => quote!(),
                };
                quote!(Self::#variant_name #pattern => { #(visitor.enqueue(#bindings);)* })
            });
            quote!(match self { #(#variants),* })
        }
        Data::Union(_) => {
            return syn::Error::new_spanned(
                name,
                "unions require explicit retained-memory accounting",
            )
            .into_compile_error()
            .into();
        }
    };
    quote! {
        impl #implementation ::etas_core::memory::RetainedSize for #name #arguments #bounds {
            fn may_retain_heap() -> bool {
                false #(|| <#field_types as ::etas_core::memory::RetainedSize>::may_retain_heap())*
            }
            fn visit_retained<#lifetime>(&#lifetime self, visitor: &mut ::etas_core::memory::MemoryVisitor<#lifetime>) {
                #body
            }
        }
    }.into()
}
