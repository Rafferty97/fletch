pub fn main() {
    let input = std::env::args().nth(1).expect("no input");
    let ast = fletch::parse(&input).unwrap();
    let result = fletch::eval(&ast).unwrap();
    println!("{:?}", result);
}
