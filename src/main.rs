use std::fs;
use oxc_allocator::Allocator;
mod deobfuscator;
mod payload;

fn main() {
    let src: String = fs::read_to_string("src/assets/input_3.js").expect("not found");
    let allocator = Allocator::default();
    let ast  = deobfuscator::deobfuscator::deobfuscate(src.as_str(), &allocator);

    let (dynamic_num, functions, ast) = payload::parser::dynamic_parser(ast, &allocator);

    println!("{}", dynamic_num);

    payload::parser::parse_payload_keys(&ast, &functions, &allocator);
}
