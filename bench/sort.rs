use std::time::{SystemTime, UNIX_EPOCH};

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state >> 33
}

fn main() {
    let mut state = seed();
    let mut xs: Vec<i64> = Vec::with_capacity(1_000_000);
    for _ in 0..1_000_000 {
        xs.push((next(&mut state) % 1_000_000_000) as i64);
    }
    xs.sort();
    println!("{} {} {}", xs[0], xs[500_000], xs[999_999]);
}
