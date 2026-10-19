fn main() {
    let mut s = String::new();
    for _ in 0..100_000 {
        s = s + "x";
    }
    println!("{}", s.len());
}
