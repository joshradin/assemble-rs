use super::AssembleFormatter;
use fmt::Write;
pub use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::iter::{Cycle, FusedIterator};
use std::marker::PhantomData;
use std::ops::{RangeInclusive, RangeToInclusive};
use std::vec;

/// Produces a text list
#[derive(Debug)]
pub struct TextListFactory<'f, B: BulletPointFactory> {
    bullet_producer: B,
    separator: String,
    indent: String,
    commands: Vec<TextListCommand<'f>>,
}

impl<'f, B: BulletPointFactory> TextListFactory<'f, B> {
    /// Create a new text list using a writer and bullet producer
    pub fn new<F: IntoBulletPointFactory<Factory = B>>(bullet_producer: F) -> Self {
        Self {
            bullet_producer: bullet_producer.into_factory(),
            separator: "\n".to_string(),
            indent: " ".repeat(2),
            commands: vec![],
        }
    }

    /// Set the separator to use
    pub fn set_separator(&mut self, separator: &str) {
        self.separator = separator.to_string();
    }

    /// Set the indent to use
    pub fn set_indent(&mut self, indent: &str) {
        self.indent = indent.to_string();
    }

    /// Use the separator
    pub fn with_separator(mut self, separator: &str) -> Self {
        self.set_separator(separator);
        self
    }

    /// Use the indent
    pub fn with_indent(mut self, indent: &str) -> Self {
        self.set_indent(indent);
        self
    }

    pub fn element<O: ToString + 'f>(mut self, element: O) -> Self {
        self.commands.push(TextListCommand::Text(Box::new(element)));
        self
    }

    pub fn elements<O: ToString + 'f, I: IntoIterator<Item = O>>(mut self, elements: I) -> Self {
        for element in elements {
            self = self.element(element);
        }
        self
    }

    /// call the factory within a sub list
    pub fn sublist<F>(mut self, func: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        self.commands.push(TextListCommand::StartSubList);
        let mut out = (func)(self);
        out.commands.push(TextListCommand::EndSubList);
        out
    }

    /// Finishes the list,
    pub fn finish(self) -> String {
        let mut indent_c = 0;

        let make_indent = |indent_c| {
            let indent_str = &self.indent;
            indent_str.repeat(indent_c)
        };

        let mut bullets = self.bullet_producer;

        let mut buffer = String::new();

        let mut is_first = true;

        for command in self.commands {
            match command {
                TextListCommand::Text(t) => {
                    let indent = (make_indent)(indent_c);
                    let bullet = bullets.next();
                    let text = t.to_string();
                    if is_first {
                        buffer = format!("{buffer}{indent}{bullet}{text}");
                        is_first = false;
                    } else {
                        let separator = &self.separator;
                        buffer = format!("{buffer}{separator}{indent}{bullet}{text}");
                    }
                }
                TextListCommand::StartSubList => {
                    bullets.increment_level();
                    indent_c += 1;
                }
                TextListCommand::EndSubList => {
                    if !bullets.decrement_level() {
                        panic!("Attempted to decrement level, but couldn't")
                    }
                    indent_c -= 1;
                }
            }
        }

        buffer
    }
}

enum TextListCommand<'f> {
    Text(Box<dyn ToString + 'f>),
    StartSubList,
    EndSubList,
}

impl<'f> Debug for TextListCommand<'f> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TextListCommand::Text(t) => {
                write!(f, "{}", t.to_string())
            }
            TextListCommand::StartSubList => {
                write!(f, "Start sub-list")
            }
            TextListCommand::EndSubList => {
                write!(f, "end sub-list")
            }
        }
    }
}

/// A trait for produced bullet points.
pub trait BulletPointFactory: Clone {
    /// Gets the next bullet point in a sequence. Should never fail.
    fn next(&mut self) -> String;

    /// Resets the factory such that the next call of next produced the first
    fn reset(&mut self);

    /// Increment the current level
    fn increment_level(&mut self);
    /// Decrement the current level.
    ///
    /// Returns whether this operation was successful or not.
    fn decrement_level(&mut self) -> bool;

    /// Map the output of one bullet point factory into another
    fn map<F>(self, func: F) -> MappedBulletPointFactory<Self, F>
    where
        F: Fn(&str) -> String + Clone,
    {
        MappedBulletPointFactory {
            generator: self,
            map: func,
        }
    }
}


impl BulletPointFactory for char {
    fn next(&mut self) -> String {
        self.to_string()
    }

    fn reset(&mut self) {}

    fn increment_level(&mut self) {}

    fn decrement_level(&mut self) -> bool {
        true
    }
}

impl BulletPointFactory for &str {
    fn next(&mut self) -> String {
        self.to_string()
    }
    fn reset(&mut self) {}

    fn increment_level(&mut self) {}

    fn decrement_level(&mut self) -> bool {
        true
    }
}

impl BulletPointFactory for String {
    fn next(&mut self) -> String {
        self.to_string()
    }

    fn reset(&mut self) {}

    fn increment_level(&mut self) {}

    fn decrement_level(&mut self) -> bool {
        true
    }
}

/// A trait to help convert types into bullet point factories. All types that implement `BulletPointFactory`,
/// also implement this trait.
pub trait IntoBulletPointFactory {
    type Factory: BulletPointFactory;

    /// Turn this type into the factory
    fn into_factory(self) -> Self::Factory;
}

impl<B: BulletPointFactory> IntoBulletPointFactory for B {
    type Factory = B;

    fn into_factory(self) -> Self::Factory {
        self
    }
}

/// Creates a bullet point factory from a [`FusedIterator`](FusedIterator)
pub struct BulletCycle<T, S: ToString>
where
    T: Clone + FusedIterator<Item = S>,
{
    base: T,
    levels: Vec<Cycle<T>>,
    _ty: PhantomData<S>,
}

impl<T, S: ToString> Clone for BulletCycle<T, S>
where
    T: Clone + FusedIterator<Item = S>,
{
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            levels: self.levels.clone(),
            _ty: Default::default(),
        }
    }
}

impl<T, S: ToString> BulletPointFactory for BulletCycle<T, S>
where
    T: Clone + FusedIterator<Item = S>,
{
    fn next(&mut self) -> String {
        self.current().next().unwrap().to_string()
    }

    fn reset(&mut self) {
        *self.current() = self.base.clone().cycle();
    }

    fn increment_level(&mut self) {
        self.levels.push(self.base.clone().cycle());
    }

    fn decrement_level(&mut self) -> bool {
        if self.levels.is_empty() {
            false
        } else {
            self.levels.pop();
            true
        }
    }
}

impl<T, S: ToString> BulletCycle<T, S>
where
    T: Clone + FusedIterator<Item = S>,
{
    pub fn new<I: IntoIterator<IntoIter = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        Self {
            base: iter.clone(),
            levels: vec![iter.cycle()],
            _ty: PhantomData,
        }
    }

    fn current(&mut self) -> &mut Cycle<T> {
        self.levels.last_mut().unwrap()
    }
}

pub type VecBulletCycle<T> = BulletCycle<vec::IntoIter<T>, T>;

fn into_vec_bullet_cycle<T: ToString + Clone, I: IntoIterator<Item = T>>(
    fused: I,
) -> VecBulletCycle<T> {
    let vec = Vec::from_iter(fused);
    BulletCycle::new(vec)
}

impl IntoBulletPointFactory for RangeInclusive<char> {
    type Factory = VecBulletCycle<String>;

    fn into_factory(self) -> Self::Factory {
        into_vec_bullet_cycle(self.into_iter().map(|c| c.to_string()))
    }
}

impl IntoBulletPointFactory for RangeInclusive<usize> {
    type Factory = VecBulletCycle<String>;

    fn into_factory(self) -> Self::Factory {
        into_vec_bullet_cycle(self.into_iter().map(|n| n.to_string()))
    }
}

#[derive(Clone)]
pub struct MultiLevelBulletFactory<B: BulletPointFactory> {
    levels: Vec<B>,
    stack: Vec<B>,
    index: usize,
}

impl<B: BulletPointFactory> BulletPointFactory for MultiLevelBulletFactory<B> {
    fn next(&mut self) -> String {
        self.current_factory().next()
    }

    fn reset(&mut self) {
        self.index = 0;
        self.current_factory().reset();
    }

    fn increment_level(&mut self) {
        self.increment_index();
        self.stack.push(self.levels[self.index].clone());
    }

    fn decrement_level(&mut self) -> bool {
        self.decrement_index();
        self.stack.pop().is_some()
    }
}

impl<B: BulletPointFactory> MultiLevelBulletFactory<B> {
    pub fn new<It: IntoIterator<Item = Ib>, Ib: IntoBulletPointFactory<Factory = B>>(
        into_iter: It,
    ) -> Self {
        let factories: Vec<B> = into_iter
            .into_iter()
            .map(|b: Ib| b.into_factory())
            .collect();
        let start_index = factories.len() - 1;
        let mut factory = Self {
            levels: factories,
            stack: vec![],
            index: start_index,
        };
        factory.increment_level();
        factory
    }

    fn increment_index(&mut self) {
        if self.index + 1 == self.levels.len() {
            self.index = 0;
        } else {
            self.index += 1;
        }
    }

    fn decrement_index(&mut self) {
        if self.index == 0 {
            self.index = self.levels.len() - 1;
        } else {
            self.index -= 1;
        }
    }

    fn current_factory(&mut self) -> &mut B {
        self.stack.last_mut().unwrap()
    }
}

#[derive(Clone)]
pub struct MappedBulletPointFactory<B, F>
where
    B: BulletPointFactory,
    F: Fn(&str) -> String + Clone,
{
    generator: B,
    map: F,
}

impl<B, F> BulletPointFactory for MappedBulletPointFactory<B, F>
where
    B: BulletPointFactory,
    F: Fn(&str) -> String + Clone,
{
    fn next(&mut self) -> String {
        (self.map)(&self.generator.next())
    }

    fn reset(&mut self) {
        self.generator.reset()
    }

    fn increment_level(&mut self) {
        self.generator.increment_level()
    }

    fn decrement_level(&mut self) -> bool {
        self.generator.decrement_level()
    }
}

/// Display information
#[derive(Debug)]
pub struct InfoList {
    heading: String,
    points: Vec<String>,
}

impl InfoList {
    pub fn new(heading: String) -> Self {
        Self {
            heading,
            points: vec![],
        }
    }

    pub fn point(&mut self, info: String) {
        self.points.push(info)
    }
}

impl Display for InfoList {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut factory = AssembleFormatter::new(f);
        writeln!(factory.important(), "{}", self.heading)?;
        for point in &self.points {
            writeln!(factory.less_important(), "{}", point)?;
        }
        Ok(())
    }
}

/// A counting list
#[derive(Debug, Default, Clone)]
pub struct Counter {
    value: usize,
    levels: Vec<usize>
}

impl Counter {
    pub fn new(start: usize) -> Self {
        Self { value: start, levels: vec![start] }
    }
}

impl BulletPointFactory for Counter {
    fn next(&mut self) -> String {
        let current = self.levels.last_mut().unwrap();
        let v = format!("{}", current);
        *current += 1;
        v
    }

    fn reset(&mut self) {
        *self.levels.last_mut().unwrap() = 0;
    }

    fn increment_level(&mut self) {
        self.levels.push(self.value);
    }

    fn decrement_level(&mut self) -> bool {
        if self.levels.len() == 1 {
            false
        } else {
            self.levels.pop();
            true
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_list() {
        let list = TextListFactory::new('>')
            .element("point 1")
            .element("point 2")
            .sublist(|b| b.elements(["point 3", "point 4"]))
            .element("point 5")
            .finish();

        let expected = r">point 1
>point 2
  >point 3
  >point 4
>point 5";
        assert_eq!(list, expected);
    }

    #[test]
    fn basic_string_list() {
        let list = TextListFactory::new("> ")
            .element("point 1")
            .element("point 2")
            .sublist(|b| b.elements(["point 3", "point 4"]))
            .element("point 5")
            .finish();

        let expected = r"> point 1
> point 2
  > point 3
  > point 4
> point 5";
        assert_eq!(list, expected);
    }

    #[test]
    fn basic_cycle() {
        let mut cycle = BulletCycle::new('a'..='d');
        assert_eq!(cycle.next(), "a");
        assert_eq!(cycle.next(), "b");
        assert_eq!(cycle.next(), "c");
        assert_eq!(cycle.next(), "d");
        assert_eq!(cycle.next(), "a");
        assert_eq!(cycle.next(), "b");
        cycle.increment_level();
        assert_eq!(cycle.next(), "a");
        assert_eq!(cycle.next(), "b");
        assert_eq!(cycle.next(), "c");
        assert!(cycle.decrement_level());
        assert_eq!(cycle.next(), "c");
    }

    #[test]
    fn numbers_then_letters() {
        let mut bullet_factory = MultiLevelBulletFactory::new(['a'..='z', '1'..='9']);

        assert_eq!(bullet_factory.next(), "a");
        assert_eq!(bullet_factory.next(), "b");
        assert_eq!(bullet_factory.next(), "c");
        bullet_factory.increment_level();
        assert_eq!(bullet_factory.next(), "1");
        bullet_factory.increment_level();
        assert_eq!(bullet_factory.next(), "a");
        assert!(bullet_factory.decrement_level());
        assert_eq!(bullet_factory.next(), "2");
        bullet_factory.increment_level();
        assert_eq!(bullet_factory.next(), "a");
        assert!(bullet_factory.decrement_level());
        assert_eq!(bullet_factory.next(), "3");
        assert!(bullet_factory.decrement_level());
        assert_eq!(bullet_factory.next(), "d");
    }

    #[test]
    fn fancy_list() {
        let mut list = TextListFactory::new(
            MultiLevelBulletFactory::new(['a'..='z', '1'..='9']).map(|s| format!("{s}. ")),
        )
        .elements(["elem1", "elem2", "elem3"])
        .sublist(|b| {
            b.element("elem4")
                .sublist(|b| b.element("elemt5"))
                .element("elem6")
                .sublist(|b| b.element("elem7"))
                .element("elem8")
        })
        .element("elem9")
        .finish();

        assert_eq!(
            list,
            r"a. elem1
b. elem2
c. elem3
  1. elem4
    a. elemt5
  2. elem6
    a. elem7
  3. elem8
d. elem9"
        )
    }

    #[test]
    fn info_list() {
        let mut info_list = InfoList::new("Heading".to_string());
        info_list.point("small info 1".to_string());
        info_list.point("small info 2".to_string());
        println!("{}", info_list);
    }

    #[test]
    fn number_inf() {
        let mut list = Counter::new(0);
        assert_eq!(list.next(), "0");
        assert_eq!(list.next(), "1");
        assert_eq!(list.next(), "2");
        assert_eq!(list.next(), "3");
    }
}
