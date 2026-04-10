use std::io::Read;

fn main() {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read stdin");

    let bytes = input.len();
    let bpe = tiktoken_rs::cl100k_base().expect("failed to load tokenizer");
    let tokens = bpe.encode_with_special_tokens(&input).len();

    println!("{bytes}\t{tokens}");
}
