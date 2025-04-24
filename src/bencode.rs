use serde_json::{self};

pub fn decode_bencoded_string(encoded_value: &str) -> (serde_json::Value, usize) {
    let colon_index = encoded_value.find(':').unwrap();
    let length = encoded_value[..colon_index].parse::<usize>().unwrap();
    let string_start = colon_index + 1;
    let string_end = string_start + length;
    let string = &encoded_value[string_start..string_end];

    (serde_json::Value::String(string.to_string()), string_end)
}

pub fn decode_bencoded_number(encoded_value: &str) -> (serde_json::Value, usize) {
    // Format: i<number>e
    let e_index = encoded_value.find('e').unwrap();
    let number_string = &encoded_value[1..e_index]; // Skip the 'i' prefix
    let number = number_string.parse::<i64>().unwrap();

    (serde_json::Value::Number(number.into()), e_index + 1)
}

pub fn decode_bencoded_list(encoded_value: &str) -> (serde_json::Value, usize) {
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
pub fn decode_bencoded_dict(encoded_value: &str) -> (serde_json::Value, usize) {
    // Format: d<contents>e
    let mut current_index = 1; // Skip the 'd' prefix
    let mut map = serde_json::Map::new();

    while encoded_value.chars().nth(current_index).unwrap() != 'e' {
        // Dictionary keys are always strings in Bencode
        let (key_value, key_len) = decode_bencoded_string(&encoded_value[current_index..]);
        current_index += key_len;

        // Get the value for this key
        let (value, value_len) = decode_bencoded_value(&encoded_value[current_index..]);
        current_index += value_len;

        // Extract the string from the key_value and insert the key-value pair
        if let serde_json::Value::String(key_str) = key_value {
            map.insert(key_str, value);
        } else {
            panic!("Dictionary key must be a string");
        }
    }

    (serde_json::Value::Object(map), current_index + 1) // +1 for the 'e' suffix
}

pub fn decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, usize) {
    match encoded_value.chars().next().unwrap() {
        '0'..='9' => decode_bencoded_string(encoded_value),
        'i' => decode_bencoded_number(encoded_value),
        'l' => decode_bencoded_list(encoded_value),
        'd' => decode_bencoded_dict(encoded_value),
        _ => panic!("Unhandled encoded value: {}", encoded_value),
    }
}
