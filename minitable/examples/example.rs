use minitable::MiniTable;

#[derive(Debug, Clone, MiniTable)]
#[minitable(module = port)]
#[minitable(index(fields(node, direction, offset), getter = find, unique))]
#[minitable(index(fields(node, direction), getter = get_by_node))]
#[minitable(index(fields(edge), getter = get_by_edge))]
pub struct Port {
    node: u32,
    direction: Direction,
    offset: u16,
    edge: u32,
}

#[derive(Debug, Clone, MiniTable)]
#[minitable(module = node)]
#[minitable(index(fields(parent), getter = get_children))]
pub struct Node {
    parent: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    Input = 0,
    Output = 1,
}

#[derive(Debug, Clone, Default)]
pub struct Graph {
    ports: port::Table,
    nodes: node::Table,
}

pub fn main() {
    let mut table = port::Table::new();

    let a = table.insert(Port {
        node: 0,
        direction: Direction::Input,
        offset: 0,
        edge: 0,
    });

    let b = table.insert(Port {
        node: 0,
        direction: Direction::Output,
        offset: 1,
        edge: 1,
    });

    let c = table.insert(Port {
        node: 0,
        direction: Direction::Input,
        offset: 2,
        edge: 1,
    });

    println!("{:#?}", table);
    // println!("{:#?}", table.get_by_node_offset(0, 0).collect::<Vec<_>>());
    table.remove(b);
    println!("{:#?}", table);
    // println!("{:#?}", table.get_by_node_offset(0, 0).collect::<Vec<_>>());
    // table.remove(a);
    // println!("{:#?}", table.get_by_node_offset(0, 0).collect::<Vec<_>>());
    // table.remove(c);
    // println!("{:#?}", table.get_by_node_offset(0, 0).collect::<Vec<_>>());
}
