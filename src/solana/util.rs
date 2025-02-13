use rand::Rng;

pub fn generate_random_seed() -> String {
    // Generate 16 random bytes
    let random_bytes: Vec<u8> = (0..16).map(|_| rand::thread_rng().gen::<u8>()).collect();

    // Encode to base58
    bs58::encode(random_bytes).into_string()
}
