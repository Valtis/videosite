use std::io::{stdin};

use scrypt::{
    password_hash::{PasswordHash, PasswordVerifier, PasswordHasher, SaltString, rand_core::OsRng},
    Scrypt,
};

// Use release build for performance, debug build is VERY slow
fn main() {
    let mut password = String::new();
    println!("Enter password to hash (no whitespace!):");
    stdin().read_line(&mut password).expect("Failed to read line");
    let password = password.trim();

    if password.is_empty() {
        println!("Password cannot be empty.");
        return;
    }

    let salt = SaltString::generate(OsRng);
    let password_hash = Scrypt
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();

    println!("Generated password hash: {}", password_hash);
    println!("Verification result: {}", 
        Scrypt.verify_password(password.as_bytes(), &PasswordHash::new(&password_hash).expect("Failed to parse password hash")).is_ok()
    );
}
