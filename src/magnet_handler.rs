#[derive(Debug, Clone)]
struct MagnetLink {
    info_hash: String,
    tr: Option<String>,
    dn: Option<String>,
}

impl MagnetLink {
    pub fn new() -> Self {
        MagnetLink {
            info_hash: String::new(),
            tr: None,
            dn: None,
        }
    }
}
pub fn magnet_parse_handler(magnet_link: String) {
    let query_index = magnet_link.find("?").expect("No query found");
    assert!(query_index == 7);

    let query = &magnet_link[query_index + 1..];
    let query_params = query.split("&");
    let mut magnet_link = MagnetLink::new();

    for param in query_params {
        let key_value = param.split("=").collect::<Vec<&str>>();
        match key_value[0] {
            "xt" => {
                let xt_parts = key_value[1].split(":").collect::<Vec<&str>>();
                magnet_link.info_hash = xt_parts.last().unwrap().to_string();
            }
            "tr" => {
                magnet_link.tr = Some(urlencoding::decode(key_value[1]).unwrap().to_string());
            }
            "dn" => magnet_link.dn = Some(key_value[1].to_string()),
            _ => {}
        }
    }

    println!("Info Hash: {}", magnet_link.info_hash);
    println!("Tracker URL:{}", magnet_link.tr.unwrap());
}
