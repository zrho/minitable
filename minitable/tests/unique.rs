use minitable::MiniTable;

#[derive(Debug, Clone, MiniTable)]
#[minitable(index(fields(value), unique))]
pub struct Item {
    value: usize,
}
