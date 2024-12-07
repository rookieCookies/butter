use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(ImmutableData)]
pub fn immutable(ts: TokenStream) -> TokenStream {
    let strct : syn::ItemStruct = syn::parse(ts).unwrap();

    let name = strct.ident;
    let gens = strct.generics;
    let mut field_names = vec![];
    let mut field_tys = vec![];
    strct.fields.iter().for_each(|x| {
        for a in x.attrs.iter() {
            let Some(ident) = a.meta.path().get_ident()
            else { continue };

            if ident.to_string() == "ignored" {
                return;
            }
        }
        field_names.push(x.ident.as_ref().unwrap());
        field_tys.push(&x.ty);
    });
    quote! {
        impl #gens #name #gens {
            #(
                #[inline(always)]
                pub fn #field_names(self) -> #field_tys {
                    self.#field_names
                }
            )*
        }
    }.into()
}


#[proc_macro_derive(Builder)]
pub fn builder(ts: TokenStream) -> TokenStream {
    let strct : syn::ItemStruct = syn::parse(ts).unwrap();

    let name = &strct.ident;
    let gens = &strct.generics;
    let mut field_names = vec![];
    let mut field_tys = vec![];
    strct.fields.iter().for_each(|x| {
        for a in x.attrs.iter() {
            let Some(ident) = a.meta.path().get_ident()
            else { continue };

            if ident.to_string() == "ignore" {
                return;
            }
        }
        field_names.push(x.ident.as_ref().unwrap());
        field_tys.push(&x.ty);
    });
    quote! {
        impl #gens #name #gens {
            #(
                #[inline(always)]
                pub fn #field_names(mut self, val: #field_tys) -> Self {
                    self.#field_names = val;
                    self
                }
            )*
        }
    }.into()
}

