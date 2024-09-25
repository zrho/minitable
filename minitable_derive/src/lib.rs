use std::collections::HashMap;

use darling::{util::PathList, FromDeriveInput, FromField, FromMeta};
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

#[proc_macro_derive(MiniTable, attributes(minitable))]
pub fn mini_table_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let output = impl_mini_table(&input);
    output.unwrap().into()
}

fn impl_mini_table(ast: &syn::DeriveInput) -> syn::Result<TokenStream> {
    let options = match MiniTableOptions::from_derive_input(&ast) {
        Ok(options) => options,
        Err(err) => return Ok(err.write_errors()),
    };

    let ident = &options.ident;
    let table = &options.table;

    let fields = options.data.take_struct().unwrap();
    let mut field_types = HashMap::new();

    for field in fields {
        let path = syn::Path::from(field.ident.clone().unwrap());
        field_types.insert(path, field.ty.clone());
    }

    let mut index_field_decl = Vec::new();
    let mut index_fields: Vec<syn::Ident> = Vec::new();
    let mut index_getters: Vec<syn::Ident> = Vec::new();
    let mut index_groups = Vec::new();
    let mut index_keys = Vec::new();
    let mut index_types = Vec::new();
    let mut index_idents = Vec::new();

    for (i, index) in options.indices.iter().enumerate() {
        let fields = &index.fields;

        index_idents.push(fields.iter().collect::<Vec<_>>());

        let index_field = syn::Ident::new(&format!("index_{}", i), ast.span());
        index_fields.push(index_field.clone());

        let index_getter = index
            .getter
            .clone()
            .unwrap_or_else(|| syn::Ident::new(&format!("get_by_{}", i), ast.span()));
        // let index_getter = syn::Ident::new(&format!("get_by_{}", i), ast.span()).into();
        index_getters.push(index_getter);

        let mut types = Vec::<syn::Type>::new();

        for field in fields.iter() {
            let ty = field_types.get(field).unwrap();
            types.push(ty.clone());
        }

        index_types.push(types.clone());

        index_field_decl.push(quote! {
            pub #index_field: ::std::collections::HashMap<(#(#types),*), (usize, usize)>
        });

        index_groups.push(quote! {
            #index_field: [usize; 2]
        });

        index_keys.push(quote! {
            (#(item.#fields.clone()),*)
        })
    }

    Ok(quote! {
        pub mod #table {
            use super::#ident;

            #[derive(Clone, Debug)]
            pub struct Table {
                pub store: ::slab::Slab<Row>,
                #(#index_field_decl),*
            }

            impl Table {
                pub fn new() -> Self {
                    Self {
                        store: ::slab::Slab::new(),
                        #(#index_fields: ::std::collections::HashMap::new()),*
                    }
                }

                #[inline]
                pub fn get(&mut self, id: usize) -> Option<&#ident> {
                    self.store.get(id).map(|row| &row.item)
                }

                #(
                    pub fn #index_getters(&self, #(#index_idents: #index_types),*) -> impl Iterator<Item = usize> + '_ {
                        let mut next = self.#index_fields.get(&(#(#index_idents),*)).map(|(id, count)| (*id, *count - 1));
                        ::std::iter::from_fn(move || {
                            let (id, count) = next?;
                            next = count.checked_sub(1).map(|count| {
                                (self.store[id].#index_fields[1], count)
                            });
                            Some(id)
                        })
                    }
                )*

                pub fn insert(&mut self, item: #ident) -> usize {
                    let id = self.store.vacant_key();

                    let mut row = Row {
                        item: item.clone(),
                        #(#index_fields: [id, id]),*
                    };

                    #(
                        match self.#index_fields.entry(#index_keys) {
                            ::std::collections::hash_map::Entry::Vacant(entry) => {
                                entry.insert((id, 1));
                            }
                            ::std::collections::hash_map::Entry::Occupied(mut entry) => {
                                let (other, count) = *entry.get();
                                let other_prev = self.store[other].#index_fields[0];
                                self.store[other].#index_fields[0] = id;
                                self.store[other_prev].#index_fields[1] = id;
                                row.#index_fields[0] = other_prev;
                                row.#index_fields[1] = other;
                                entry.insert((id, count + 1));
                            }
                        }
                    )*

                    self.store.insert(row)
                }

                pub fn remove(&mut self, id: usize) -> Option<#ident> {
                    todo!()
                }
            }

            #[derive(Clone, Debug)]
            pub struct Row {
                item: #ident,
                #(#index_groups),*
            }
        }
    })
}

#[derive(FromDeriveInput)]
#[darling(attributes(minitable), supports(struct_named))]
struct MiniTableOptions {
    ident: syn::Ident,
    data: darling::ast::Data<(), FieldOptions>,
    table: syn::Ident,
    #[darling(default, multiple, rename = "index")]
    indices: Vec<IndexAttr>,
}

#[derive(FromMeta)]
struct IndexAttr {
    fields: PathList,
    #[darling(default)]
    getter: Option<syn::Ident>,
}

#[derive(FromField)]
struct FieldOptions {
    ident: Option<syn::Ident>,
    ty: syn::Type,
}
