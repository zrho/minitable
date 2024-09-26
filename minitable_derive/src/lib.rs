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

    Ok(quote! {
        pub mod #module {
            use super::*;

            #[derive(Clone, Default)]
            pub struct Table {
                pub store: ::slab::Slab<Row>,
                #(pub #multi_index_fields: ::ahash::AHashMap<(#(#multi_index_types),*), (usize, usize)>,)*
            }

            impl Table {
                /// Create a new empty table.
                #[inline]
                pub fn new() -> Self {
                    Self::default()
                }

                /// Get a reference to an item by its id.
                #[inline]
                pub fn get(&mut self, id: usize) -> Option<&#ident> {
                    self.store.get(id).map(|row| &row.item)
                }

                #(
                    /// Iterate over items in the table by an index lookup.
                    #[inline]
                    pub fn #multi_index_getters(&self, #(#multi_index_idents: #multi_index_types),*) -> impl ::std::iter::ExactSizeIterator<Item = usize> + '_ {
                        struct Iter<'a> {
                            table: &'a Table,
                            next: Option<(usize, usize)>,
                        }

                        impl<'a> ::std::iter::Iterator for Iter<'a> {
                            type Item = usize;

                            fn next(&mut self) -> Option<Self::Item> {
                                let (id, count) = self.next?;
                                self.next = count.checked_sub(1).map(|count| {
                                    (self.table.store[id].#multi_index_fields[1], count)
                                });
                                Some(id)
                            }

                            fn size_hint(&self) -> (usize, Option<usize>) {
                                (0, Some(self.next.map(|(_, count)| count + 1).unwrap_or(0)))
                            }
                        }

                        impl<'a> ::std::iter::ExactSizeIterator for Iter<'a> {}

                        let mut next = self.#multi_index_fields.get(&(#(#multi_index_idents),*)).map(|(id, count)| (*id, *count - 1));
                        Iter { table: self, next }
                    }
                )*

                /// Insert a new item into the table and return its id.
                pub fn insert(&mut self, item: #ident) -> usize {
                    let id = self.store.vacant_key();

                    let mut row = Row {
                        item: item.clone(),
                        #(#multi_index_fields: [id, id]),*
                    };

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

                    self.store.insert(row)
                }

                /// Remove the item with the given id from the table.
                ///
                /// Returns the removed item if it was present, or `None` otherwise.
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

                    Some(item)
                }

                /// Returns `true` if the table contains an item with the given id.
                #[inline]
                pub fn contains(&self, id: usize) -> bool {
                    self.store.contains(id)
                }

                /// Returns the number of items in the table.
                #[inline]
                pub fn len(&self) -> usize {
                    self.store.len()
                }

                /// Remove all items from the table.
                pub fn clear(&mut self) {
                    self.store.clear();
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
