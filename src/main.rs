use serde_json;
use std::env;

fn decode_bencoded_string(encoded_value: &str) -> (serde_json::Value, usize) {
    let colon_index = encoded_value.find(':').unwrap();
    let number_string = &encoded_value[..colon_index];
    let number = number_string.parse::<i64>().unwrap();
    let string = &encoded_value[colon_index + 1..colon_index + 1 + number as usize];
    return (
        serde_json::Value::String(string.to_string()),
        colon_index + 1 + number as usize,
    );
}

fn decode_bencoded_number(encoded_value: &str) -> (serde_json::Value, usize) {
    let i_index = encoded_value.find('i').unwrap();
    let e_index = encoded_value.find('e').unwrap();
    let number_string = &encoded_value[i_index + 1..e_index];
    let number = number_string.parse::<i64>().unwrap();
    return (serde_json::Value::Number(number.into()), e_index + 1);
}

fn decode_bencoded_list(encoded_value: &str) -> (serde_json::Value, usize) {
    let l_index = encoded_value.find('l').unwrap();

    let mut current_index = l_index + 1;
    let mut values = Vec::new();
    while encoded_value.chars().nth(current_index).unwrap() != 'e' {
        let (value, value_len) = decode_bencoded_value(&encoded_value[current_index..]);
        println!("value: {} and length: {}", value, value_len);
        values.push(value);
        current_index += value_len;
    }

    return (serde_json::Value::Array(values), current_index + 1);
}

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, usize) {
    // If encoded_value starts with a digit, it's a number
    if encoded_value.chars().next().unwrap().is_digit(10) {
        return decode_bencoded_string(encoded_value);
    } else if encoded_value.starts_with('i') {
        // Example: i123e -> 123
        return decode_bencoded_number(encoded_value);
    } else if encoded_value.starts_with('l') {
        // Example: l123e -> [123]
        return decode_bencoded_list(encoded_value);
    } else {
        panic!("Unhandled encoded value: {}", encoded_value)
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: your_bittorrent.sh decode <encoded_value>");
        return;
    }

    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        eprintln!("Logs from your program will appear here!");

        // Uncomment this block to pass the first stage
        let encoded_value = &args[2];
        let (decoded_value, _) = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value.to_string());
    } else {
        println!("unknown command: {}", args[1])
    }
}
