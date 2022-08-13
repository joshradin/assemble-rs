//! Control attribute based flow for attribute selection

use crate::immutable::Immutable;
use crate::named::Named;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::marker::PhantomData;

/// Some attribute
pub trait Attribute: PartialEq {
    fn attribute_id(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }
}

pub struct AttributeSchema<T: Attribute> {
    compatibility: AttributeCompatibilityChain<T>,
    disambiguation: MultipleCandidatesChain<T>,
}

impl<T: Attribute> AttributeSchema<T> {
    pub const fn new() -> Self {
        Self {
            compatibility: AttributeCompatibilityChain::new(),
            disambiguation: MultipleCandidatesChain::new(),
        }
    }

    pub fn compatibility(&self) -> &AttributeCompatibilityChain<T> {
        &self.compatibility
    }
    pub fn disambiguation(&self) -> &MultipleCandidatesChain<T> {
        &self.disambiguation
    }

    pub fn compatibility_mut(&mut self) -> &mut AttributeCompatibilityChain<T> {
        &mut self.compatibility
    }
    pub fn disambiguation_mut(&mut self) -> &mut MultipleCandidatesChain<T> {
        &mut self.disambiguation
    }

    /// Attempt to find the matching producer for a given consumer
    pub fn find_match<'a, I: IntoIterator<Item = &'a Named<T>>>(
        &self,
        consumer: &'a Named<T>,
        producers: I,
    ) -> Option<&'a Named<T>> {
        let mut compat_producers: Vec<&Named<T>> = producers
            .into_iter()
            .filter(|producer: &&'a Named<T>| {
                self.compatibility().is_compatible(producer, consumer)
            })
            .collect();
        println!("compat: {:?}", compat_producers);
        match compat_producers.len() {
            0 => None,
            1 => Some(compat_producers.remove(0)),
            _ => self
                .disambiguation()
                .try_disambiguate(consumer, compat_producers),
        }
    }
}

pub struct AttributeCompatibilityChain<T: Attribute> {
    rules: Vec<Box<dyn AttributeCompatibilityRule<T>>>,
}

impl<T: Attribute> AttributeCompatibilityChain<T> {
    pub const fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a compatibility rule
    pub fn add<R: AttributeCompatibilityRule<T> + 'static>(&mut self, rule: R) {
        self.rules.push(Box::new(rule));
    }

    /// Add the [PartialOrd](PartialOrd) as a compatibility rule, where Some(Equals) and Some(Less) is compatible
    pub fn ordered(&mut self)
    where
        T: PartialOrd,
    {
        self.add(|check: &mut CompatibilityCheck<T>| {
            let consumer = check.consumer();
            let producer = check.producer();
            match consumer.partial_cmp(producer) {
                Some(Ordering::Equal) | Some(Ordering::Less) => check.compatible(),
                Some(Ordering::Greater) => check.incompatible(),
                None => {}
            };
        })
    }

    /// Add the [PartialOrd](PartialOrd) as a compatibility rule, where Some(Equals) and Some(Greater) is compatible
    pub fn ordered_rev(&mut self)
    where
        T: PartialOrd,
    {
        self.add(|check: &mut CompatibilityCheck<T>| {
            let consumer = check.consumer();
            let producer = check.producer();
            match consumer.partial_cmp(producer) {
                Some(Ordering::Equal) | Some(Ordering::Greater) => check.compatible(),
                Some(Ordering::Less) => check.incompatible(),
                None => {}
            };
        })
    }

    /// Check if two attributes are compatible.
    ///
    /// Will check all registered rules, and if none specify [`compatible()`](CompatibilityCheck::compatible) or
    /// [`incompatible()`](CompatibilityCheck::incompatible).
    pub fn is_compatible(&self, producer: &Named<T>, consumer: &Named<T>) -> bool {
        let mut check = CompatibilityCheck::new(consumer, producer);
        for rule in &self.rules {
            rule.check_capability(&mut check);
            if let Some(compatability) = check.is_compatible {
                return compatability;
            }
        }
        producer == consumer
    }
}

pub trait AttributeCompatibilityRule<T: Attribute> {
    /// Check the capability of an attribute
    fn check_capability(&self, check: &mut CompatibilityCheck<T>);
}

impl<F, T> AttributeCompatibilityRule<T> for F
where
    for<'a> F: Fn(&'a mut CompatibilityCheck<T>),
    T: Attribute,
{
    fn check_capability(&self, check: &mut CompatibilityCheck<T>) {
        (self)(check)
    }
}

/// Shorthand for adding compatiblity
pub struct IsCompatible<T: Attribute> {
    _ty: PhantomData<T>,
    consumer: String,
    producer: Vec<String>,
}

impl<T: Attribute> AttributeCompatibilityRule<T> for IsCompatible<T> {
    fn check_capability(&self, check: &mut CompatibilityCheck<T>) {
        if self.consumer == check.consumer().name()
            && self.producer.contains(&check.producer().name().to_string())
        {
            check.compatible()
        }
    }
}

impl<T: Attribute> IsCompatible<T> {
    pub fn new<'a>(consumer: &str, producer: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            _ty: PhantomData,
            consumer: consumer.to_string(),
            producer: producer.into_iter().map(str::to_string).collect(),
        }
    }
}

/// data to pass compatibility check information
pub struct CompatibilityCheck<'a, T: Attribute> {
    consumer: &'a Named<T>,
    producer: &'a Named<T>,
    is_compatible: Option<bool>,
}

impl<'a, T: Attribute> CompatibilityCheck<'a, T> {
    fn new(consumer: &'a Named<T>, producer: &'a Named<T>) -> Self {
        Self {
            consumer,
            producer,
            is_compatible: None,
        }
    }
}

impl<'a, T: Attribute> CompatibilityCheck<'a, T> {
    /// Get the consumer attribute
    pub fn consumer(&self) -> &Named<T> {
        self.consumer
    }

    /// Get the producer attribute
    pub fn producer(&self) -> &Named<T> {
        self.producer
    }

    /// Calling this method will indicate that the attributes are compatible
    pub fn compatible(&mut self) {
        self.is_compatible = Some(true);
    }

    /// Calling this method will indicate that the attributes are incompatible
    pub fn incompatible(&mut self) {
        self.is_compatible = Some(false);
    }
}

pub trait MultipleCandidatesRule<T: Attribute> {
    /// Try to disambiguate
    fn disambiguate(&self, details: &mut MultipleCandidates<T>);
}

impl<F, T> MultipleCandidatesRule<T> for F
where
    for<'a> F: Fn(&'a mut MultipleCandidates<T>),
    T: Attribute,
{
    fn disambiguate(&self, details: &mut MultipleCandidates<T>) {
        (self)(details)
    }
}

pub struct MultipleCandidatesChain<T: Attribute> {
    chain: Vec<Box<dyn MultipleCandidatesRule<T>>>,
}

impl<T: Attribute> MultipleCandidatesChain<T> {
    pub const fn new() -> Self {
        Self { chain: Vec::new() }
    }

    pub fn add<R: MultipleCandidatesRule<T> + 'static>(&mut self, rule: R) {
        self.chain.push(Box::new(rule));
    }

    pub fn try_disambiguate<'a, I>(
        &self,
        consumer: &'a Named<T>,
        candidates: I,
    ) -> Option<&'a Named<T>>
    where
        I: IntoIterator<Item = &'a Named<T>>,
        T: 'a,
    {
        let mut details = MultipleCandidates::new(consumer, candidates.into_iter().collect());
        for rule in &self.chain {
            rule.disambiguate(&mut details);
            if let Some(closest) = details.closest_match {
                return Some(closest);
            }
        }
        None
    }
}

pub struct MultipleCandidates<'a, T: Attribute> {
    consumer_value: &'a Named<T>,
    candidate_values: Vec<&'a Named<T>>,
    closest_match: Option<&'a Named<T>>,
}

impl<'a, T: Attribute> MultipleCandidates<'a, T> {
    fn new(consumer_value: &'a Named<T>, candidate_values: Vec<&'a Named<T>>) -> Self {
        Self {
            consumer_value,
            candidate_values,
            closest_match: None,
        }
    }

    pub fn consumer_value(&self) -> &Named<T> {
        self.consumer_value
    }

    pub fn candidate_values(&self) -> &[&'a Named<T>] {
        &self.candidate_values[..]
    }

    pub fn closest_match(&mut self, value: &'a Named<T>) {
        self.closest_match = Some(value);
    }
}

pub struct Equality;

impl<T: Attribute> MultipleCandidatesRule<T> for Equality {
    fn disambiguate(&self, multiple: &mut MultipleCandidates<T>) {
        let mut closest = None;
        for prod in multiple.candidate_values() {
            if *prod == multiple.consumer_value() {
                closest = Some(*prod);
                break;
            }
        }
        if let Some(closest) = closest {
            multiple.closest_match(closest);
        }
    }
}

#[macro_export]
macro_rules! named_attribute {
    ($attribute_type:ident, $make:expr, $name:ident) => {
        const $name: once_cell::sync::Lazy<Named<$attribute_type>> =
            once_cell::sync::Lazy::new(|| Named::new(stringify!($name), ($make)));
    };
    ($attribute_type:ident, $name:ident) => {
        $crate::named_attribute!($attribute_type, $attribute_type, $name);
    };
    ($name:ident) => {
        $crate::named_attribute!(Self, $name);
    };
}

/// Container of [`Attribute`s](Attribute)
#[derive(Debug, Clone)]
pub struct AttributeContainer;

/// Something that carries attributes
pub trait HasAttributes {
    fn get_attributes(&self) -> &AttributeContainer;
}

/// Something that carries attributes and is configurable
pub trait ConfigurableAttributes : HasAttributes {
    fn attributes<F : FnOnce(&mut AttributeContainer)>(&mut self, func: F);
}

#[cfg(test)]
mod tests {
    use crate::flow::attributes::{
        Attribute, AttributeSchema, CompatibilityCheck, Equality, IsCompatible, MultipleCandidates,
    };
    use crate::named::Named;
    use once_cell::sync::Lazy;

    #[derive(PartialEq, Clone, Debug)]
    pub struct Usage;

    impl Attribute for Usage {}

    impl Usage {
        named_attribute!(CLASSES);
        named_attribute!(JAR);
    }

    #[test]
    fn java_style_resolution() {
        let mut compatibility: AttributeSchema<Usage> = AttributeSchema::new();
        compatibility
            .compatibility_mut()
            .add(IsCompatible::new("CLASSES", ["CLASSES", "JAR"]));

        compatibility.disambiguation_mut().add(Equality);

        let classes = &*Usage::CLASSES;
        let jar = &*Usage::JAR;
        let compat = compatibility.find_match(classes, [jar]);
        assert!(compat.is_some());
        let compat = compat.unwrap();
        assert_eq!(compat, &*Usage::JAR);

        let compat = compatibility.find_match(classes, [jar, classes]);
        assert!(compat.is_some());
        let compat = compat.unwrap();
        assert_eq!(compat, &*Usage::CLASSES);

        let compat = compatibility.find_match(jar, [jar, classes]);
        assert!(compat.is_some());
        let compat = compat.unwrap();
        assert_eq!(compat, &*Usage::JAR);

        let compat = compatibility.find_match(jar, [classes]);
        assert!(compat.is_none());
    }
}
