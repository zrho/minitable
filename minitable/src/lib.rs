//! An in-memory database with a single table.
//!
//! The `MiniTable` derive macro creates a data structure that stores instances of a struct.
//! The table supports fast insertion, deletion, and lookup by a primary key and by specified
//! indices. The indices are updated automatically when the table is modified, ensuring that
//! they are always consistent with the table.
//!
//! Each row in a table is identified by a unique integer key. The key is assigned when a row is
//! inserted and is reused when a row is deleted. Deleting and immediately reinserting a row will
//! reuse the key.
//! A row can not be modified in place to avoid the indices becoming inconsistent. Instead,
//! the row must be removed and reinserted. We might add support for in-place modification in the
//! future.
//!
//! By default, an index is non-unique, meaning that multiple rows can have the same value for the
//! indexed fields. If the `unique` attribute is added to an index attribute, we ensure that the
//! value of the indexed fields is unique across all rows in the table.
//! This allows slightly more efficient data structures to be used for the index.
//!
//! # Example
//!
//! # Notes
//!
//!  - Struct fields are cloned when they occur in an index. It is therefore wise to use
//!    types that are cheap to clone.
//!  - Concurrent modification and durability are out of scope. Use a real database if you need those.
//!  - The table uses intrusive cyclic doubly linked list for non-unique indices. This allows us to avoid
//!    additional allocations beyond the slab and the hash tables for the indices. Moreover this enables
//!    `O(1)` removal of elements from the index.
//!  - We currently use the `ahash` crate for hashing. This might become customizable in the future.
pub use minitable_derive::MiniTable;
