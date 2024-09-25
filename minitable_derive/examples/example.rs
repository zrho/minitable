use std::collections::HashMap;

use minitable_derive::MiniTable;
use slab::Slab;

#[derive(Debug, Clone, MiniTable)]
#[minitable(table = port)]
#[minitable(index(fields(node, offset), getter = get_by_node_offset))]
#[minitable(index(fields(edge), getter = get_by_edge))]
pub struct Port {
    node: u32,
    offset: u16,
    edge: u32,
}

pub fn main() {
    let mut table = port::Table::new();

    let a = table.insert(Port {
        node: 0,
        offset: 0,
        edge: 0,
    });

    let b = table.insert(Port {
        node: 0,
        offset: 4,
        edge: 1,
    });

    let c = table.insert(Port {
        node: 0,
        offset: 0,
        edge: 1,
    });

    println!("{:#?}", table.get_by_node_offset(0, 0).collect::<Vec<_>>());
    println!("{:#?}", table.store.iter().collect::<Vec<_>>());
}
