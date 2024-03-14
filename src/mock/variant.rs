use std::{any::Any, collections::HashMap, ops::Deref};

pub trait MockDebug: std::fmt::Debug {}

impl MockDebug for MockEmpty {}
impl MockDebug for bool {}
impl MockDebug for u8 {}
impl MockDebug for i8 {}
impl MockDebug for u16 {}
impl MockDebug for i16 {}
impl MockDebug for u32 {}
impl MockDebug for i32 {}
impl MockDebug for u64 {}
impl MockDebug for i64 {}
impl MockDebug for String {}
impl<T: std::fmt::Debug> MockDebug for Vec<T> {}
impl<T: std::fmt::Debug> MockDebug for Option<T> {}
impl<K: std::fmt::Debug, V: std::fmt::Debug> MockDebug for HashMap<K, V> {}

pub trait TMockVariant: MockDebug + Any {
    fn into_mock_variant(self) -> MockVariant;
}

impl TMockVariant for bool {
    fn into_mock_variant(self) -> MockVariant {
        MockVariant::new::<bool>(self, "bool")
    }
}
impl TMockVariant for u8 {
    fn into_mock_variant(self) -> MockVariant {
        MockVariant::new::<u8>(self, "u8")
    }
}
impl TMockVariant for i8 {
    fn into_mock_variant(self) -> MockVariant {
        MockVariant::new::<i8>(self, "i8")
    }
}
impl TMockVariant for u16 {
    fn into_mock_variant(self) -> MockVariant {
        MockVariant::new::<u16>(self, "u16")
    }
}
impl TMockVariant for i16 {
    fn into_mock_variant(self) -> MockVariant {
        MockVariant::new::<i16>(self, "i16")
    }
}
impl TMockVariant for u32 {
    fn into_mock_variant(self) -> MockVariant {
        MockVariant::new::<u32>(self, "u32")
    }
}
impl TMockVariant for i32 {
    fn into_mock_variant(self) -> MockVariant {
        MockVariant::new::<i32>(self, "i32")
    }
}
impl TMockVariant for u64 {
    fn into_mock_variant(self) -> MockVariant {
        MockVariant::new::<u64>(self, "u64")
    }
}
impl TMockVariant for i64 {
    fn into_mock_variant(self) -> MockVariant {
        MockVariant::new::<i64>(self, "i64")
    }
}
impl TMockVariant for String {
    fn into_mock_variant(self) -> MockVariant {
        MockVariant::new::<String>(self.clone(), "String")
    }
}
impl<T: IntrospectType + MockDebug + 'static> TMockVariant for Option<T>
where
    T: IntrospectType + Clone,
{
    fn into_mock_variant(self) -> MockVariant {
        MockVariant::new(self, "Option ".to_string() + &T::get_type())
    }
}
impl<T: IntrospectType + MockDebug + 'static> TMockVariant for Vec<T>
where
    T: IntrospectType + Clone,
{
    fn into_mock_variant(self) -> MockVariant {
        MockVariant::new(self, "Vec ".to_string() + &T::get_type())
    }
}
impl<K: IntrospectType + MockDebug + 'static, V: IntrospectType + MockDebug + 'static> TMockVariant
    for HashMap<K, V>
where
    K: IntrospectType + Clone,
    V: IntrospectType + Clone,
{
    fn into_mock_variant(self) -> MockVariant {
        MockVariant::new(
            self,
            "HashMap ".to_string() + &K::get_type() + " " + &V::get_type(),
        )
    }
}

#[derive(Debug)]
pub struct MockVariant {
    value: Box<dyn TMockVariant>,
    kind: String,
}

impl MockVariant {
    fn empty() -> Self {
        Self::new::<MockEmpty>(MockEmpty {}, "None")
    }

    fn new<T: TMockVariant + 'static>(value: T, kind: impl Into<String>) -> Self {
        MockVariant {
            value: Box::new(value),
            kind: kind.into(),
        }
    }

    fn to_value<T: Copy>(&self, conversion_type: &'static str) -> Result<T, MockConversionError> {
        if self.kind != conversion_type {
            return Err(MockConversionError("Conversion Failed"));
        }
        unsafe { Ok(*self.to_value_unchecked::<T>()) }
    }

    fn to_value_cloned<T: Clone>(
        &self,
        conversion_type: &'static str,
    ) -> Result<&T, MockConversionError> {
        if self.kind != conversion_type {
            return Err(MockConversionError("Conversion Failed"));
        }
        unsafe { Ok(self.to_value_unchecked::<T>()) }
    }

    unsafe fn to_value_unchecked<T>(&self) -> &T {
        &*(self.value.deref() as *const dyn Any as *mut T)
    }
}

#[derive(Debug)]
pub struct MockConversionError(&'static str);

#[derive(Clone, Copy, Debug)]
pub struct MockEmpty {}

impl IntrospectType for MockEmpty {
    fn get_type() -> String {
        "None".into()
    }
}

impl TMockVariant for MockEmpty {
    fn into_mock_variant(self) -> MockVariant {
        MockVariant::new::<MockEmpty>(self, "None")
    }
}

pub trait IntrospectType {
    fn get_type() -> String;
}

impl IntrospectType for bool {
    fn get_type() -> String {
        "bool".into()
    }
}

impl IntrospectType for u8 {
    fn get_type() -> String {
        "u8".into()
    }
}

impl IntrospectType for i8 {
    fn get_type() -> String {
        "i8".into()
    }
}

impl IntrospectType for u16 {
    fn get_type() -> String {
        "u16".into()
    }
}

impl IntrospectType for i16 {
    fn get_type() -> String {
        "i16".into()
    }
}

impl IntrospectType for u32 {
    fn get_type() -> String {
        "u32".into()
    }
}

impl IntrospectType for i32 {
    fn get_type() -> String {
        "i32".into()
    }
}

impl IntrospectType for u64 {
    fn get_type() -> String {
        "u64".into()
    }
}

impl IntrospectType for i64 {
    fn get_type() -> String {
        "i64".into()
    }
}

impl IntrospectType for String {
    fn get_type() -> String {
        "String".into()
    }
}

impl<T: IntrospectType> IntrospectType for Option<T> {
    fn get_type() -> String {
        "Option".to_string() + " " + &T::get_type()
    }
}

impl<T: IntrospectType> IntrospectType for Vec<T> {
    fn get_type() -> String {
        "Vec".to_string() + " " + &T::get_type()
    }
}

impl<K: IntrospectType, V: IntrospectType> IntrospectType for HashMap<K, V> {
    fn get_type() -> String {
        "HashMap".to_string() + " " + &K::get_type() + " " + &V::get_type()
    }
}

#[test]
fn test_i32() {
    let mock = 5.into_mock_variant();
    assert_eq!(mock.kind, "i32".to_string());
    assert_eq!(mock.to_value::<i32>("i32").unwrap(), 5);
}

#[test]
fn test_option() {
    let mock = Some(10).into_mock_variant();
    assert_eq!(mock.kind, "Option i32".to_string());
    assert_eq!(
        mock.to_value::<Option<i32>>("Option i32").unwrap(),
        Some(10)
    );
}

#[test]
fn test_vec() {
    let mock = vec![3, 2, 4, 5, 10].into_mock_variant();
    assert_eq!(mock.kind, "Vec i32".to_string());
    assert_eq!(
        mock.to_value_cloned::<Vec<i32>>("Vec i32").unwrap().clone(),
        vec![3, 2, 4, 5, 10]
    );
}

#[test]
fn test_hashmap() {
    let mut map = HashMap::new();
    map.insert("Something".to_string(), 20);
    let mock = map.into_mock_variant();

    let mut testmap = HashMap::new();
    testmap.insert("Something".to_string(), 20);

    assert_eq!(mock.kind, "HashMap String i32".to_string());
    assert_eq!(
        mock.to_value_cloned::<HashMap<String, i32>>("HashMap String i32")
            .unwrap()
            .clone(),
        testmap
    );
}

#[test]
fn test_conversion_fail() {
    let mock = "hello".to_string().into_mock_variant();
    assert_eq!(mock.kind, "String".to_string());
    assert!(mock.to_value_cloned::<i32>("Not String").is_err());
}
