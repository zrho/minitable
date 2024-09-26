use criterion::{black_box, criterion_group, criterion_main, Criterion};
use minitable::MiniTable;

#[derive(Debug, Clone, MiniTable)]
#[minitable(module = edge)]
pub struct Edge {
    source: u32,
    target: u32,
}

pub fn insert_fully_connected(c: &mut Criterion) {
    c.bench_function("insert_fully_connected", |b| {
        b.iter(|| {
            let mut table = edge::Table::new();

            for i in 0..100 {
                for j in 0..100 {
                    table.insert(Edge {
                        source: i,
                        target: j,
                    });
                }
            }

            black_box(table);
        })
    });
}

criterion_group!(benches, insert_fully_connected);
criterion_main!(benches);
