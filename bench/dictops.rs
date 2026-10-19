use std::collections::HashMap;

fn main() {
    let mut d: HashMap<i64, i64> = HashMap::new();
    for i in 0..1_000_000i64 {
        d.insert(i % 100_000, i);
    }
    let mut total: i64 = 0;
    for i in 0..1_000_000i64 {
        total += *d.get(&(i % 100_000)).unwrap_or(&0);
    }
    println!("{}", total);
}
