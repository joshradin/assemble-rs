//! Provides ways to "fingerprint" something

pub const FINGER_PRINT_SIZE: usize = 32;

pub trait Fingerprint<const FINGER_PRINT: usize = FINGER_PRINT_SIZE> {
    fn fingerprint(&self) -> [u8; FINGER_PRINT];

    fn matches_fingerprint(&self, other: &[u8]) -> bool {
        self.fingerprint() == other
    }
}
