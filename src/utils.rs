use rand::{rng, Rng};
pub fn generate_peer_id() -> String {
    let mut rng = rng();

    (0..20)
        .map(|_| rng.random_range(0..10).to_string())
        .collect()
}
