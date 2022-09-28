//! Defines the types of data that can be stored in a packet

use byteorder::{BigEndian, ByteOrder};
use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::hash::Hash;
use std::string::FromUtf8Error;
use either::Either;
use generic_array::{ArrayLength, GenericArray};

/// A value which can be written into data
pub trait IntoData {
    /// Serialize the data
    fn serialize(&self) -> Vec<u8>;
}

/// A value which can read from a data stream
pub trait FromData: Sized {
    type Err;

    /// Deserialize data by taking bytes. The bytes deque is guaranteed to
    /// have minimum size amount of data, and at most the maximum size if
    /// maximum size returns `Some(_)`.
    fn deserialize(bytes: &mut VecDeque<u8>) -> Result<Self, Self::Err>;

    /// The minimum amount of bytes required to deserialize this type
    fn minimum_size() -> usize;

    /// The max amount of bytes required to deserialize this type, if known
    fn maximum_size() -> Option<usize> {
        None
    }
}

impl<T : IntoData> IntoData for &T {
    fn serialize(&self) -> Vec<u8> {
        (**self).serialize()
    }
}

impl<T1 : IntoData, T2 : IntoData> IntoData for (T1, T2) {
    fn serialize(&self) -> Vec<u8> {
        let mut output = vec![];
        output.extend(self.0.serialize());
        output.extend(self.1.serialize());
        output
    }
}

impl<T1 : FromData, T2 : FromData> FromData for (T1, T2) {
    type Err = Either<T1::Err, T2::Err>;

    fn deserialize(bytes: &mut VecDeque<u8>) -> Result<Self, Self::Err> {
        let t1 = T1::deserialize(bytes).map_err(Either::Left)?;
        let t2 = T2::deserialize(bytes).map_err(Either::Right)?;
        Ok((t1, t2))
    }

    fn minimum_size() -> usize {
        T1::minimum_size() + T2::minimum_size()
    }
}



impl IntoData for u8 {
    fn serialize(&self) -> Vec<u8> {
        vec![*self]
    }
}

impl FromData for u8 {
    type Err = Infallible;

    fn deserialize(bytes: &mut VecDeque<u8>) -> Result<Self, Self::Err> {
        Ok(bytes.pop_front().unwrap())
    }

    fn minimum_size() -> usize {
        1
    }
}

impl IntoData for bool {
    fn serialize(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

impl FromData for bool {
    type Err = Infallible;

    fn deserialize(bytes: &mut VecDeque<u8>) -> Result<Self, Self::Err> {
        Ok(bytes.pop_front().unwrap() == 1)
    }

    fn minimum_size() -> usize {
        1
    }
}

impl IntoData for u16 {
    fn serialize(&self) -> Vec<u8> {
        Vec::from_iter(self.to_be_bytes())
    }
}

impl FromData for u16 {
    type Err = Infallible;

    fn deserialize(bytes: &mut VecDeque<u8>) -> Result<Self, Self::Err> {
        let bytes = bytes.drain(..2).collect::<Vec<_>>();
        let mut buffer = [0; 2];
        buffer.clone_from_slice(&bytes);
        Ok(u16::from_be_bytes(buffer))
    }

    fn minimum_size() -> usize {
        2
    }

    fn maximum_size() -> Option<usize> {
        Some(2)
    }
}

impl IntoData for u64 {
    fn serialize(&self) -> Vec<u8> {
        Vec::from_iter(self.to_be_bytes())
    }
}

impl FromData for u64 {
    type Err = Infallible;

    fn deserialize(bytes: &mut VecDeque<u8>) -> Result<Self, Self::Err> {
        let bytes = bytes.drain(..8).collect::<Vec<_>>();
        let mut buffer = [0; 8];
        buffer.clone_from_slice(&bytes);
        Ok(u64::from_be_bytes(buffer))
    }

    fn minimum_size() -> usize {
        8
    }

    fn maximum_size() -> Option<usize> {
        Some(8)
    }
}

impl<T: IntoData> IntoData for Vec<T> {
    fn serialize(&self) -> Vec<u8> {
        self.as_slice().serialize()
    }
}

impl<T: FromData> FromData for Vec<T> {
    type Err = T::Err;

    fn deserialize(bytes: &mut VecDeque<u8>) -> Result<Self, Self::Err> {
        let length = u64::deserialize(bytes).unwrap();
        let mut output = vec![];
        for _ in 0..length {
            output.push(T::deserialize(bytes)?);
        }
        Ok(output)
    }

    fn minimum_size() -> usize {
        8
    }
}

impl<T: IntoData, const N: usize> IntoData for [T; N] {
    fn serialize(&self) -> Vec<u8> {
        let mut output = vec![];
        output.extend((self.len() as u64).serialize());
        for element in self {
            output.extend(element.serialize())
        }
        output
    }
}

impl<T: IntoData> IntoData for &[T] {
    fn serialize(&self) -> Vec<u8> {
        let mut output = vec![];
        output.extend((self.len() as u64).serialize());
        for element in *self {
            output.extend(element.serialize())
        }
        output
    }
}

impl<T: FromData + Default + Copy, const N: usize> FromData for [T; N] {
    type Err = T::Err;

    fn deserialize(bytes: &mut VecDeque<u8>) -> Result<Self, Self::Err> {
        let length = u64::deserialize(bytes).unwrap();
        let mut output = [T::default(); N];
        for i in 0..length {
            output[i as usize] = T::deserialize(bytes)?;
        }
        Ok(output)
    }

    fn minimum_size() -> usize {
        8
    }
}

impl IntoData for &str {
    fn serialize(&self) -> Vec<u8> {
        self.bytes().collect::<Vec<u8>>().serialize()
    }
}

impl IntoData for String {
    fn serialize(&self) -> Vec<u8> {
        self.as_str().serialize()
    }
}

impl FromData for String {
    type Err = FromUtf8Error;

    fn deserialize(bytes: &mut VecDeque<u8>) -> Result<Self, Self::Err> {
        let vec: Vec<u8> = deserialize(bytes).unwrap();
        String::from_utf8(vec)
    }

    fn minimum_size() -> usize {
        <Vec<u8>>::minimum_size()
    }
}

impl<K : IntoData + Eq + Hash, V : IntoData> IntoData for HashMap<K, V> {
    fn serialize(&self) -> Vec<u8> {
        self
            .iter()
            .collect::<Vec<_>>()
            .serialize()
    }
}

impl<K : FromData + Eq + Hash, V : FromData> FromData for HashMap<K, V> {
    type Err = <Vec<(K, V)> as FromData>::Err;

    fn deserialize(bytes: &mut VecDeque<u8>) -> Result<Self, Self::Err> {
        let inner: Vec<(K, V)> = deserialize(bytes)?;
        Ok(HashMap::from_iter(inner))
    }

    fn minimum_size() -> usize {
        <Vec<(K, V)> as FromData>::minimum_size()
    }
}

impl <T : IntoData> IntoData for Option<T> {
    fn serialize(&self) -> Vec<u8> {
        match self {
            None => {
                vec![0]
            }
            Some(data) => {
                let mut output = vec![1];
                output.extend(data.serialize());
                output
            }
        }
    }
}

impl <T: FromData> FromData for Option<T> {
    type Err = T::Err;

    fn deserialize(bytes: &mut VecDeque<u8>) -> Result<Self, Self::Err> {
        let discriminant = bytes.pop_front().unwrap();
        match discriminant == 0{
            true => {
                Ok(None)
            },
            false => {
                let value = T::deserialize(bytes)?;
                Ok(Some(value))
            }
        }
    }

    fn minimum_size() -> usize {
        1 // if None, only one byte is needed
    }

    fn maximum_size() -> Option<usize> {
        T::maximum_size().map(|size| size + 1)
    }
}


impl<T : IntoData, A : ArrayLength<T>> IntoData for GenericArray<T, A> {
    fn serialize(&self) -> Vec<u8> {
        Vec::from_iter(self).serialize()
    }
}

impl<T : FromData, A : ArrayLength<T>> FromData for GenericArray<T, A> {
    type Err = T::Err;

    fn deserialize(bytes: &mut VecDeque<u8>) -> Result<Self, Self::Err> {
        let result = Vec::deserialize(bytes)?;
        Ok(GenericArray::from_exact_iter(result).unwrap())
    }

    fn minimum_size() -> usize {
        <Vec<T>>::minimum_size()
    }
}

/// Serialize some data
pub fn serialize<T : IntoData>(data: &T) -> Vec<u8> {
    data.serialize()
}

/// Serialize some data
pub fn deserialize<T : FromData>(data: &mut VecDeque<u8>) -> Result<T, T::Err> {
    <T as FromData>::deserialize(data)
}



#[cfg(test)]
mod tests {
    use crate::{FromData, IntoData};
    use std::collections::{HashMap, VecDeque};
    use crate::data::{deserialize, serialize};

    #[test]
    fn serialize_u64() {
        let value = 3099_u64;

        let ref mut buffer = VecDeque::from_iter(value.serialize());
        let deserialized = u64::deserialize(buffer).unwrap();

        assert_eq!(deserialized, value);
    }

    #[test]
    fn serialize_u64_array() {
        let value = [1_u64, 2, 3, 4];

        let ref mut buffer = VecDeque::from_iter(value.serialize());
        let deserialized: [u64; 4] = deserialize(buffer).unwrap();

        assert_eq!(&deserialized, &value);
    }

    #[test]
    fn serialize_u64_vector() {
        let value = vec![1_u64, 2, 3, 4];

        let ref mut buffer = VecDeque::from_iter(value.serialize());
        let deserialized: Vec<u64> = deserialize(buffer).unwrap();
        assert!(buffer.is_empty());

        assert_eq!(&deserialized, &value);
    }

    #[test]
    fn serialize_string() {
        let value = "Hello, World!".to_string();

        let ref mut buffer = VecDeque::from_iter(serialize(&value));
        let deserialized: String = deserialize(buffer).unwrap();
        assert!(buffer.is_empty());

        assert_eq!(deserialized, value);
    }

    // allow to test for more complex types
    #[test]
    fn serialize_string_vec() {
        let value = vec![
            format!("Hello"),
            format!("World"),
            format!("Goodbye"),
            format!("World"),
        ];

        let ref mut buffer = VecDeque::from_iter(serialize(&value));
        let deserialized: Vec<String> = deserialize(buffer).unwrap();
        assert!(buffer.is_empty());

        assert_eq!(deserialized, value);
    }

    #[test]
    fn serialize_tuple() {
        let value = (String::from("Hello, World"), vec![1_u64, 2, 3]);

        let ref mut buffer = VecDeque::from_iter(serialize(&value));
        let deserialized: (String, Vec<u64>) = deserialize(buffer).unwrap();
        assert!(buffer.is_empty());

        assert_eq!(deserialized, value);
    }

    #[test]
    fn serialize_map() {
        let mut value = HashMap::new();
        value.insert("Josh".to_string(), 23_u64);
        value.insert("Annie".to_string(), 20);
        value.insert("Erika".to_string(), 52);
        value.insert("Tony".to_string(), 54);

        let ref mut buffer = VecDeque::from_iter(serialize(&value));
        let deserialized: HashMap<String, u64> = deserialize(buffer).unwrap();
        assert!(buffer.is_empty());

        assert_eq!(deserialized, value);
    }

    #[test]
    fn serialize_option() {
        let mut value = Some("3".to_string());

        let ref mut buffer = VecDeque::from_iter(serialize(&value));
        let deserialized: Option<String> = deserialize(buffer).unwrap();
        assert!(buffer.is_empty());

        assert_eq!(deserialized, value);

        value = None;

        let ref mut buffer = VecDeque::from_iter(serialize(&value));
        let deserialized: Option<String> = deserialize(buffer).unwrap();
        assert!(buffer.is_empty());

        assert_eq!(deserialized, value);
    }
}
