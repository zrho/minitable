use minitable::MiniTable;
use std::fmt::Debug;

#[derive(Debug, Clone, MiniTable)]
#[minitable(index(fields(value), unique))]
pub struct Item {
    value: usize,
}
