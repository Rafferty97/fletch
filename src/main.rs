pub fn main() {
    let input = std::env::args().nth(1).expect("no input");
    let ast = fletch::parse(&input).unwrap();
    println!("{:#?}", ast);
}
