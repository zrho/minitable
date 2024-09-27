//! The implementation of the `MiniTable` proc-macro.
use darling::{util::PathList, FromDeriveInput, FromField, FromMeta};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
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

    let fields = options.data.take_struct().unwrap();
    let mut field_types = HashMap::new();

    for field in fields {
        let path = syn::Path::from(field.ident.clone().unwrap());
        field_types.insert(path, field.ty.clone());
    }

    let mut multi_index_fields: Vec<syn::Ident> = Vec::new();
    let mut multi_index_getters: Vec<syn::Ident> = Vec::new();
    let mut multi_index_remove: Vec<syn::Ident> = Vec::new();
    let mut multi_index_drain: Vec<syn::Ident> = Vec::new();
    let mut multi_index_types = Vec::new();
    let mut multi_index_idents = Vec::new();

    for index in options.indices.iter() {
        if index.unique {
            continue;
        }

        let fields = &index.fields;

        multi_index_idents.push(fields.iter().collect::<Vec<_>>());
        multi_index_fields.push((format_ident!("index_{}", index.fields().join("_"))).clone());
        multi_index_getters.push(format_ident!("get_by_{}", index.fields().join("_")));
        multi_index_remove.push(format_ident!("remove_by_{}", index.fields().join("_")));
        multi_index_drain.push(format_ident!("drain_by_{}", index.fields().join("_")));

        multi_index_types.push(
            index
                .fields
                .iter()
                .map(|field| field_types.get(field).unwrap().clone())
                .collect::<Vec<_>>(),
        );
    }

    let mut unique_index_fields: Vec<syn::Ident> = Vec::new();
    let mut unique_index_getters: Vec<syn::Ident> = Vec::new();
    let mut unique_index_remove: Vec<syn::Ident> = Vec::new();
    let mut unique_index_types = Vec::new();
    let mut unique_index_idents = Vec::new();

    for index in options.indices.iter() {
        if !index.unique {
            continue;
        }

        let fields = &index.fields;

        unique_index_idents.push(fields.iter().collect::<Vec<_>>());
        unique_index_fields.push((format_ident!("index_{}", index.fields().join("_"))).clone());
        unique_index_getters.push(format_ident!("get_by_{}", index.fields().join("_")));
        unique_index_remove.push(format_ident!("remove_by_{}", index.fields().join("_")));

        unique_index_types.push(
            index
                .fields
                .iter()
                .map(|field| field_types.get(field).unwrap().clone())
                .collect::<Vec<_>>(),
        );
    }

    let table_type = format_ident!("{}Table", ident);
    let row_type = format_ident!("{}Row", ident);

    Ok(quote! {
        #[derive(Clone, Default)]
        pub struct #table_type {
            store: ::slab::Slab<#row_type>,
            #(#multi_index_fields: ::ahash::AHashMap<(#(#multi_index_types),*), (u32, u32)>,)*
            #(#unique_index_fields: ::ahash::AHashMap<(#(#unique_index_types),*), u32>,)*
        }

        impl #table_type {
            /// Create a new empty table.
            #[inline]
            pub fn new() -> Self {
                Self::default()
            }

            /// Get a reference to an item by its id.
            #[inline]
            pub fn get(&self, id: usize) -> Option<&#ident> {
                self.store.get(id).map(|row| &row.item)
            }

            #(
                /// Iterate over items in the table by an index lookup.
                #[inline]
                pub fn #multi_index_getters(&self, #(#multi_index_idents: #multi_index_types),*) -> impl ::std::iter::ExactSizeIterator<Item = usize> + '_ {
                    struct Iter<'a> {
                        table: &'a #table_type,
                        count: usize,
                        next: Option<u32>,
                    }

                    impl<'a> ::std::iter::Iterator for Iter<'a> {
                        type Item = usize;

                        fn next(&mut self) -> Option<Self::Item> {
                            let id = self.next? as usize;
                            self.count = self.count.checked_sub(1)?;
                            self.next = Some(self.table.store[id].#multi_index_fields[1]);
                            Some(id)
                        }

                        fn size_hint(&self) -> (usize, Option<usize>) {
                            (0, Some(self.count))
                        }
                    }

                    impl<'a> ::std::iter::ExactSizeIterator for Iter<'a> {}

                    match self.#multi_index_fields.get(&(#(#multi_index_idents),*)) {
                        Some((id, count)) => Iter { table: self, next: Some(*id), count: *count as usize },
                        None => Iter { table: self, next: None, count: 0 }
                    }
                }
            )*

            #(
                #[inline]
                pub fn #unique_index_getters(&self, #(#unique_index_idents: #unique_index_types),*) -> Option<usize> {
                    Some(*self.#unique_index_fields.get(&(#(#unique_index_idents),*))? as usize)
                }
            )*

            #(
                pub fn #multi_index_drain(&mut self, #(#multi_index_idents: #multi_index_types),*) -> impl ::std::iter::ExactSizeIterator<Item = (usize, #ident)> + '_ {
                    struct Drain<'a> {
                        table: &'a mut #table_type,
                        count: usize,
                        next: Option<u32>,
                    }

                    impl<'a> ::std::iter::Iterator for Drain<'a> {
                        type Item = (usize, #ident);

                        fn next(&mut self) -> Option<Self::Item> {
                            let id = self.next? as usize;
                            self.count = self.count.checked_sub(1)?;
                            self.next = Some(self.table.store[id].#multi_index_fields[1]);
                            let item = self.table.remove(id).unwrap();
                            Some((id, item))
                        }

                        fn size_hint(&self) -> (usize, Option<usize>) {
                            (0, Some(self.count))
                        }
                    }

                    impl<'a> ::std::iter::ExactSizeIterator for Drain<'a> {}

                    match self.#multi_index_fields.get(&(#(#multi_index_idents),*)) {
                        Some((id, count)) => Drain { next: Some(*id), count: *count as usize, table: self, },
                        None => Drain { table: self, next: None, count: 0 }
                    }
                }
            )*

            #(
                pub fn #multi_index_remove(&mut self, #(#multi_index_idents: #multi_index_types),*) {
                    for _ in self.#multi_index_drain(#(#multi_index_idents),*) {}
                }
            )*

            #(
                pub fn #unique_index_remove(&mut self, #(#unique_index_idents: #unique_index_types),*) -> Option<#ident> {
                    let id = self.#unique_index_getters(#(#unique_index_idents),*)?;
                    self.remove(id)
                }
            )*

            /// Insert a new item into the table and return its id.
            pub fn insert(&mut self, item: #ident) -> usize {
                self.try_insert(item).expect("insert failed")
            }

            /// Insert a new item into the table and return its id, if possible.
            pub fn try_insert(&mut self, item: #ident) -> Option<usize> {
                let id = self.store.vacant_key();

                let mut row = #row_type {
                    item: item.clone(),
                    #(#multi_index_fields: [id as u32, id as u32]),*
                };

                // For the unique indices, we must verify that inserting the item does not break uniqueness.
                // To avoid having to do rollbacks, we first collect all the entries for the unique indices.
                #(
                    let #unique_index_fields = match self.#unique_index_fields.entry((#(item.#unique_index_idents.clone()),*)) {
                        ::std::collections::hash_map::Entry::Vacant(entry) => entry,
                        ::std::collections::hash_map::Entry::Occupied(_) => return None,
                    };
                )*

                // Once we have vacant entries for all unique indices, we can safely insert the item.
                #(
                    #unique_index_fields.insert(id as u32);
                )*

                #(
                    match self.#multi_index_fields.entry((#(item.#multi_index_idents.clone()),*)) {
                        ::std::collections::hash_map::Entry::Vacant(entry) => {
                            entry.insert((id as u32, 1));
                        }
                        ::std::collections::hash_map::Entry::Occupied(mut entry) => {
                            let (other, count) = *entry.get();
                            let other_prev = self.store[other as usize].#multi_index_fields[0];
                            self.store[other as usize].#multi_index_fields[0] = id as u32;
                            self.store[other_prev as usize].#multi_index_fields[1] = id as u32;
                            row.#multi_index_fields[0] = other_prev;
                            row.#multi_index_fields[1] = other;
                            entry.insert((id as u32, count + 1));
                        }
                    }
                )*

                Some(self.store.insert(row))
            }

            /// Remove the item with the given id from the table.
            ///
            /// Returns the removed item if it was present, or `None` otherwise.
            pub fn remove(&mut self, id: usize) -> Option<#ident> {
                let row = self.store.try_remove(id)?;
                let item = row.item;

                #(
                    self.#unique_index_fields.remove(&(#(item.#unique_index_idents.clone()),*));
                )*

                #(
                    let [prev, next] = row.#multi_index_fields;

                    if prev as usize == id {
                        self.#multi_index_fields.remove(&(#(item.#multi_index_idents.clone()),*));
                    } else {
                        self.store[prev as usize].#multi_index_fields[1] = next;
                        self.store[next as usize].#multi_index_fields[0] = prev;
                        let entry = self.#multi_index_fields.get_mut(&(#(item.#multi_index_idents.clone()),*)).unwrap();
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
                    self.#unique_index_fields.clear();
                )*
                #(
                    self.#multi_index_fields.clear();
                )*
            }
        }

        impl std::fmt::Debug for #table_type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                struct Helper<'a>(&'a #table_type);

                impl std::fmt::Debug for Helper<'_> {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.debug_map().entries(self.0.store.iter()).finish()
                    }
                }

                f.debug_tuple("Table").field(&Helper(self)).finish()
            }
        }

        impl ::std::iter::FromIterator<#ident> for #table_type {
            fn from_iter<T: ::std::iter::IntoIterator<Item = #ident>>(iter: T) -> Self {
                let mut table = Self::new();

                for item in iter {
                    table.insert(item);
                }

                table
            }
        }

        impl ::std::ops::Index<usize> for #table_type {
            type Output = #ident;

            fn index(&self, index: usize) -> &Self::Output {
                &self.store[index].item
            }
        }

        #[derive(Clone, Debug)]
        struct #row_type {
            item: #ident,
            #(#multi_index_fields: [u32; 2]),*
        }
    })
}

#[derive(FromDeriveInput)]
#[darling(attributes(minitable), supports(struct_named))]
struct MiniTableOptions {
    ident: syn::Ident,
    data: darling::ast::Data<(), FieldOptions>,
    #[darling(default, multiple, rename = "index")]
    indices: Vec<IndexAttr>,
}

#[derive(FromMeta)]
struct IndexAttr {
    fields: PathList,
    #[darling(default)]
    unique: bool,
}

impl IndexAttr {
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
