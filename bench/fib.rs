fn fib(n: i64) -> i64 {
    if n < 2 {
        return n;
    }
    fib(n - 1) + fib(n - 2)
}

fn main() {
    let mut total: i64 = 0;
    for i in 0..32 {
        total += fib(i);
    }
    println!("{}", total);
}
