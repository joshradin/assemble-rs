//! Provides the serialization and deserialization functions used within this project.


#[cfg(feature = "ron")]
mod ron;
#[cfg(feature = "ron")]
pub use ron::*;

#[cfg(feature = "compact")]
mod compact;
#[cfg(feature = "compact")]
pub use compact::*;

#[cfg(not(any(feature = "ron", feature = "compact")))]
mod json;
#[cfg(not(any(feature = "ron", feature = "compact")))]
pub use json::*;


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use more_collection_macros::map;
    use super::*;

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    struct TestStruct {
        a: i32,
        b: String,
    }

    #[test]
    fn can_serialize_generics() {
        let test_struct = TestStruct { a: 15, b: "Hello, World".to_string() };
        let serialized = Serializable::new(test_struct.clone()).unwrap();
        println!("serialized: {}", to_string(&serialized).unwrap());
        let value: TestStruct = serialized.deserialize().unwrap();
        assert_eq!(value, test_struct);
    }
}
