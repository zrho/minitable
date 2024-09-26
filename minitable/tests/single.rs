use minitable::MiniTable;

macro_rules! assert_eq_sorted {
    ($left:expr, $right:expr) => {
        let mut left: Vec<_> = $left.into_iter().collect();
        let mut right: Vec<_> = $right.into_iter().collect();
        left.sort();
        right.sort();
        assert_eq!(left, right);
    };
}

#[derive(Debug, Clone, PartialEq, Eq, MiniTable)]
#[minitable(index(fields(value)))]
pub struct Item {
    pub value: u32,
}

#[test]
pub fn test_remove() {
    let mut table = ItemTable::new();

    let a = table.insert(Item { value: 0 });
    let b = table.insert(Item { value: 1 });
    let c = table.insert(Item { value: 0 });
    let d = table.insert(Item { value: 0 });

    assert_eq_sorted!(table.get_by_value(0), [a, d, c]);
    table.remove(d);
    assert_eq_sorted!(table.get_by_value(0), [a, c]);
    table.remove(c);
    assert_eq_sorted!(table.get_by_value(0), [a]);
    table.remove(a);
    assert_eq_sorted!(table.get_by_value(0), []);
}

#[test]
pub fn test_remove_reinsert() {
    let mut table = ItemTable::new();

    let a = table.insert(Item { value: 0 });
    let b = table.insert(Item { value: 1 });
    let c = table.insert(Item { value: 0 });
    let d = table.insert(Item { value: 0 });

    assert_eq_sorted!(table.get_by_value(0), [a, d, c]);
    let item = table.remove(c).unwrap();
    assert_eq!(table.remove(c), None);
    assert_eq_sorted!(table.get_by_value(0), [a, d]);
    assert_eq!(table.insert(item), c);
    assert_eq_sorted!(table.get_by_value(0), [a, d, c]);
}
