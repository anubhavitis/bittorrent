use serde_json;
use std::collections::HashMap;
use std::env;

fn decode_bencoded_string(encoded_value: &str) -> (serde_json::Value, usize) {
    let colon_index = encoded_value.find(':').unwrap();
    let length = encoded_value[..colon_index].parse::<usize>().unwrap();
    let string_start = colon_index + 1;
    let string_end = string_start + length;
    let string = &encoded_value[string_start..string_end];

    (serde_json::Value::String(string.to_string()), string_end)
}

fn decode_bencoded_number(encoded_value: &str) -> (serde_json::Value, usize) {
    // Format: i<number>e
    let e_index = encoded_value.find('e').unwrap();
    let number_string = &encoded_value[1..e_index]; // Skip the 'i' prefix
    let number = number_string.parse::<i64>().unwrap();

    (serde_json::Value::Number(number.into()), e_index + 1)
}

fn decode_bencoded_list(encoded_value: &str) -> (serde_json::Value, usize) {
    // Format: l<contents>e
    let mut current_index = 1; // Skip the 'l' prefix
    let mut values = Vec::new();

    while encoded_value.chars().nth(current_index).unwrap() != 'e' {
        let (value, value_len) = decode_bencoded_value(&encoded_value[current_index..]);
        values.push(value);
        current_index += value_len;
    }

    (serde_json::Value::Array(values), current_index + 1) // +1 for the 'e' suffix
}

fn decode_bencoded_dict(encoded_value: &str) -> (serde_json::Value, usize) {
    // Format: d<contents>e
    let mut current_index = 1; // Skip the 'd' prefix
    let mut dict_values = HashMap::new();

    while encoded_value.chars().nth(current_index).unwrap() != 'e' {
        let (key, key_len) = decode_bencoded_string(&encoded_value[current_index..]);
        let (value, value_len) = decode_bencoded_value(&encoded_value[current_index + key_len..]);
        dict_values.insert(key, value);
        current_index += key_len + value_len;
    }

    (
        serde_json::Value::Object(
            dict_values
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        ),
        current_index + 1,
    ) // +1 for the 'e' suffix
}

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, usize) {
    match encoded_value.chars().next().unwrap() {
        '0'..='9' => decode_bencoded_string(encoded_value),
        'i' => decode_bencoded_number(encoded_value),
        'l' => decode_bencoded_list(encoded_value),
        'd' => decode_bencoded_dict(encoded_value),
        _ => panic!("Unhandled encoded value: {}", encoded_value),
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
