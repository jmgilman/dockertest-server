use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::iter;

pub enum ConnectionType {
    EXTERNAL,
    INTERNAL,
}

/// Generates a random string of characters of the given length.
pub fn rand_string(length: usize) -> String {
    let mut rng = thread_rng();
    iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .map(char::from)
        .take(length)
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_rand_string() {
        let result = super::rand_string(10);
        assert_eq!(result.len(), 10);
    }
}
