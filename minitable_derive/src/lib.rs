//! The implementation of the `MiniTable` proc-macro.
use darling::{util::PathList, FromDeriveInput, FromField, FromMeta};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;

#[proc_macro_derive(MiniTable, attributes(minitable))]
pub fn mini_table_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let output = impl_mini_table(&input);
    output.unwrap().into()
}

fn impl_mini_table(ast: &syn::DeriveInput) -> syn::Result<TokenStream> {
    let options = match MiniTableOptions::from_derive_input(ast) {
        Ok(options) => options,
        Err(err) => return Ok(err.write_errors()),
    };

    let ident = &options.ident;
    let module = &options.module;

    let fields = options.data.take_struct().unwrap();
    let mut field_types = HashMap::new();

    for field in fields {
        let path = syn::Path::from(field.ident.clone().unwrap());
        field_types.insert(path, field.ty.clone());
    }

    let mut multi_index_fields: Vec<syn::Ident> = Vec::new();
    let mut multi_index_getters: Vec<syn::Ident> = Vec::new();
    let mut multi_index_keys = Vec::new();
    let mut multi_index_types = Vec::new();
    let mut multi_index_idents = Vec::new();

    for index in options.indices.iter() {
        if index.unique {
            continue;
        }

        let fields = &index.fields;

        multi_index_idents.push(fields.iter().collect::<Vec<_>>());

        let index_field = index.index_field();
        multi_index_fields.push(index_field.clone());
        multi_index_getters.push(index.getter());

        multi_index_types.push(
            index
                .fields
                .iter()
                .map(|field| field_types.get(field).unwrap().clone())
                .collect::<Vec<_>>(),
        );

        multi_index_keys.push(quote! {
            (#(item.#fields.clone()),*)
        })
    }

    let mut unique_index_fields = Vec::new();
    let mut unique_index_types = Vec::new();
    let mut unique_index_getters: Vec<syn::Ident> = Vec::new();
    let mut unique_index_idents = Vec::new();
    let mut unique_index_keys = Vec::new();

    for index in options.indices.iter() {
        if !index.unique {
            continue;
        }

        let fields = &index.fields;

        unique_index_idents.push(index.fields.iter().collect::<Vec<_>>());
        unique_index_fields.push(index.index_field());
        unique_index_getters.push(index.getter());
        unique_index_types.push(
            index
                .fields
                .iter()
                .map(|field| field_types.get(field).unwrap().clone())
                .collect::<Vec<_>>(),
        );
        unique_index_keys.push(quote! {
            (#(item.#fields.clone()),*)
        });
    }

    Ok(quote! {
        pub mod #module {
            use super::*;

            #[derive(Clone, Default)]
            pub struct Table {
                pub store: ::slab::Slab<Row>,
                #(pub #multi_index_fields: ::ahash::AHashMap<(#(#multi_index_types),*), (usize, usize)>,)*
                #(pub #unique_index_fields: ::ahash::AHashMap<(#(#unique_index_types),*), usize>,)*
            }

            impl Table {
                #[inline]
                pub fn new() -> Self {
                    Self::default()
                }

                #[inline]
                pub fn get(&mut self, id: usize) -> Option<&#ident> {
                    self.store.get(id).map(|row| &row.item)
                }

                #(
                    pub fn #multi_index_getters(&self, #(#multi_index_idents: #multi_index_types),*) -> impl Iterator<Item = usize> + '_ {
                        let mut next = self.#multi_index_fields.get(&(#(#multi_index_idents),*)).map(|(id, count)| (*id, *count - 1));
                        ::std::iter::from_fn(move || {
                            let (id, count) = next?;
                            next = count.checked_sub(1).map(|count| {
                                (self.store[id].#multi_index_fields[1], count)
                            });
                            Some(id)
                        })
                    }
                )*

                #(
                    pub fn #unique_index_getters(&self, #(#unique_index_idents: #unique_index_types),*) -> Option<usize> {
                        self.#unique_index_fields.get(&(#(#unique_index_idents),*)).copied()
                    }
                )*

                #[inline]
                pub fn insert(&mut self, item: #ident) -> usize {
                    self.try_insert(item).expect("uniqueness violation")
                }

                pub fn try_insert(&mut self, item: #ident) -> Option<usize> {
                    let id = self.store.vacant_key();

                    let mut row = Row {
                        item: item.clone(),
                        #(#multi_index_fields: [id, id]),*
                    };

                    // Check uniqueness
                    #(
                        if self.#unique_index_fields.contains_key(&#unique_index_keys) {
                            return None;
                        }
                    )*

                    // Create unique indices
                    #(
                        self.#unique_index_fields.insert(#unique_index_keys, id);
                    )*

                    // Create multi indices
                    #(
                        match self.#multi_index_fields.entry(#multi_index_keys) {
                            ::std::collections::hash_map::Entry::Vacant(entry) => {
                                entry.insert((id, 1));
                            }
                            ::std::collections::hash_map::Entry::Occupied(mut entry) => {
                                let (other, count) = *entry.get();
                                let other_prev = self.store[other].#multi_index_fields[0];
                                self.store[other].#multi_index_fields[0] = id;
                                self.store[other_prev].#multi_index_fields[1] = id;
                                row.#multi_index_fields[0] = other_prev;
                                row.#multi_index_fields[1] = other;
                                entry.insert((id, count + 1));
                            }
                        }
                    )*

                    Some(self.store.insert(row))
                }

                pub fn remove(&mut self, id: usize) -> Option<#ident> {
                    let row = self.store.try_remove(id)?;
                    let item = row.item;

                    #(
                        let [prev, next] = row.#multi_index_fields;

                        if prev == id {
                            self.#multi_index_fields.remove(&#multi_index_keys);
                        } else {
                            self.store[prev].#multi_index_fields[1] = next;
                            self.store[next].#multi_index_fields[0] = prev;
                            let entry = self.#multi_index_fields.get_mut(&#multi_index_keys).unwrap();
                            *entry = (prev, entry.1 - 1);
                        }
                    )*

                    #(
                        self.#unique_index_fields.remove(&#unique_index_keys);
                    )*

                    Some(item)
                }

                #[inline]
                pub fn contains(&self, id: usize) -> bool {
                    self.store.contains(id)
                }

                #[inline]
                pub fn len(&self) -> usize {
                    self.store.len()
                }

                pub fn clear(&mut self) {
                    self.store.clear();
                    #(
                        self.#unique_index_fields.clear();
                    )*
                    #(
                        self.#multi_index_fields.clear();
                    )*
                }
            }

            impl std::fmt::Debug for Table {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    struct Helper<'a>(&'a Table);

                    impl std::fmt::Debug for Helper<'_> {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                            f.debug_map().entries(self.0.store.iter()).finish()
                        }
                    }

                    f.debug_tuple("Table").field(&Helper(self)).finish()
                }
            }

            impl ::std::iter::FromIterator<#ident> for Table {
                fn from_iter<T: ::std::iter::IntoIterator<Item = #ident>>(iter: T) -> Self {
                    let mut table = Self::new();

                    for item in iter {
                        table.insert(item);
                    }

                    table
                }
            }

            #[derive(Clone, Debug)]
            pub struct Row {
                item: #ident,
                #(#multi_index_fields: [usize; 2]),*
            }
        }
    })
}

#[derive(FromDeriveInput)]
#[darling(attributes(minitable), supports(struct_named))]
struct MiniTableOptions {
    ident: syn::Ident,
    data: darling::ast::Data<(), FieldOptions>,
    module: syn::Ident,
    #[darling(default, multiple, rename = "index")]
    indices: Vec<IndexAttr>,
}

#[derive(FromMeta)]
struct IndexAttr {
    fields: PathList,
    #[darling(default)]
    getter: Option<syn::Ident>,
    #[darling(default)]
    unique: bool,
}

impl IndexAttr {
    pub fn getter(&self) -> syn::Ident {
        self.getter.clone().unwrap_or_else(|| {
            syn::Ident::new(
                &format!("get_by_{}", self.fields().join("_")),
                proc_macro2::Span::call_site(),
            )
        })
    }

    pub fn index_field(&self) -> syn::Ident {
        syn::Ident::new(
            &format!("index_{}", self.fields().join("_")),
            proc_macro2::Span::call_site(),
        )
    }

    pub fn fields(&self) -> Vec<String> {
        self.fields
            .iter()
            .map(|field| field.get_ident().unwrap().to_string())
            .collect()
    }
}

#[derive(FromField)]
struct FieldOptions {
    ident: Option<syn::Ident>,
    ty: syn::Type,
}
