fn main() {
    let mut total: i64 = 0;
    for i in 0..100_000_000i64 {
        total += i;
    }
    println!("{}", total);
}
