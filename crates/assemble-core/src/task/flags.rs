//! Add flags for tasks

use log::{error, info};
use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;
use std::error::Error;
use std::marker::PhantomData;
use std::ops::Deref;
use std::str::FromStr;
use uuid::Uuid;

use crate::{ok, Task};

/// A Flag request is a given flag and an optional list of strings
pub struct OptionRequest<T> {
    flag: String,
    values: Option<Vec<T>>,
}

impl<T> OptionRequest<T> {
    /// The name of the flag
    pub fn flag(&self) -> &str {
        &self.flag
    }

    /// The values in the flag.
    ///
    /// If no value is taken, then `None` returned.
    pub fn values(&self) -> Option<&[T]> {
        self.values.as_ref().map(|vals| &vals[..])
    }
}

/// A flag declaration defines how a task should be executed.
///
/// All tasks will return a list of flag declarations, which by default is empty. In addition,
/// tasks are required to respond to all flag requests.
pub struct OptionDeclaration {
    flag: String,
    help: String,
    takes_value: bool,
    allow_multiple_values: bool,
    optional: bool,
    flag_type: TypeId,
    parse_value: Option<Box<dyn Fn(&str) -> Result<Box<dyn Any>, Box<dyn Error + Send + Sync>>>>,
    verify_value: Option<Box<dyn Fn(&str) -> Result<(), Box<dyn Error + Send + Sync>>>>,
}

impl OptionDeclaration {
    pub fn flag(&self) -> &str {
        &self.flag
    }
    pub fn help(&self) -> &str {
        &self.help
    }
    pub fn takes_value(&self) -> bool {
        self.takes_value
    }

    pub fn is_flag(&self) -> bool {
        !self.takes_value
    }

    pub fn allow_multiple_values(&self) -> bool {
        self.allow_multiple_values
    }
    pub fn optional(&self) -> bool {
        self.optional
    }
}

pub struct OptionDeclarations {
    task_type: String,
    declarations: HashMap<String, OptionDeclaration>,
}

impl OptionDeclarations {
    pub fn new<T: Task, I: IntoIterator<Item = OptionDeclaration>>(options: I) -> Self {
        Self {
            task_type: type_name::<T>().to_string(),
            declarations: options
                .into_iter()
                .map(|opt: OptionDeclaration| (opt.flag.to_string(), opt))
                .collect(),
        }
    }

    fn new_weak(&self, map: HashMap<String, Vec<String>>) -> WeakOptionsDecoder {
        WeakOptionsDecoder {
            option_dec_string: self.task_type.clone(),
            fed_options: map,
        }
    }

    pub fn slurper(&self) -> OptionsSlurper {
        OptionsSlurper::new(self)
    }
}

impl Deref for OptionDeclarations {
    type Target = HashMap<String, OptionDeclaration>;

    fn deref(&self) -> &Self::Target {
        &self.declarations
    }
}

/// Build flag declarations
pub struct OptionDeclarationBuilder<T> {
    flag: String,
    help: Option<String>,
    takes_value: bool,
    allow_multiple_values: bool,
    optional: bool,
    parse_value: Option<Box<dyn Fn(&str) -> Result<Box<dyn Any>, Box<dyn Error + Send + Sync>>>>,
    verify_value: Option<Box<dyn Fn(&str) -> Result<(), Box<dyn Error + Send + Sync>>>>,
    _phantom: PhantomData<T>,
}

impl<T: 'static> OptionDeclarationBuilder<T> {
    pub fn new(flag: &str) -> Self {
        Self {
            flag: flag.to_string(),
            help: None,
            takes_value: true,
            allow_multiple_values: false,
            optional: false,
            parse_value: None,
            verify_value: None,
            _phantom: PhantomData,
        }
    }

    pub fn help(mut self, help: impl AsRef<str>) -> Self {
        self.help = Some(help.as_ref().to_string());
        self
    }
    pub fn takes_value(mut self, takes_value: bool) -> Self {
        self.takes_value = takes_value;
        self
    }
    pub fn allow_multiple_values(mut self, allow_multiple_values: bool) -> Self {
        if allow_multiple_values {
            self.takes_value = true;
        }
        self.allow_multiple_values = allow_multiple_values;
        self
    }
    pub fn optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }

    pub fn value_parser<F, E>(mut self, func: F) -> Self
    where
        F: Fn(&str) -> Result<T, E>,
        F: 'static,
        E: 'static + Error + Send + Sync,
    {
        let boxed: Box<(dyn Fn(&str) -> Result<Box<dyn Any>, Box<dyn Error + Send + Sync>>)> =
            Box::new(move |str| {
                let res = (func)(str);
                res.map(|t| Box::new(t) as Box<dyn Any>)
                    .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
            });
        self.parse_value = Some(boxed);
        self
    }

    pub fn build(self) -> OptionDeclaration {
        OptionDeclaration {
            flag: self.flag,
            help: self.help.unwrap_or(String::new()),
            takes_value: self.takes_value,
            allow_multiple_values: self.allow_multiple_values,
            optional: self.optional,
            flag_type: TypeId::of::<T>(),
            parse_value: (self.takes_value).then_some(()).map(|_| {
                self.parse_value
                    .expect("Value parser required for flags that take a value")
            }),
            verify_value: self.verify_value,
        }
    }
}

impl<T: FromStr + 'static> OptionDeclarationBuilder<T>
where
    <T as FromStr>::Err: Error + Send + Sync,
{
    /// Use the FromStr::from_str as the value parser
    pub fn use_from_str(self) -> Self {
        self.value_parser(T::from_str)
    }
}

impl OptionDeclarationBuilder<bool> {
    pub fn flag(flag: &str) -> Self {
        Self::new(flag).takes_value(false).optional(true)
    }
}

/// Slurps a set of options based on a given [`OptionDeclarations`](OptionDeclaration)
pub struct OptionsSlurper<'dec> {
    decs: &'dec OptionDeclarations,
}

fn flag_value_entry() -> Vec<String> {
    vec![String::new()]
}

impl<'dec> OptionsSlurper<'dec> {
    pub fn new(decs: &'dec OptionDeclarations) -> Self {
        Self { decs }
    }

    /// From a slice of strings, parses left from right. Slurped arguments are returned in a form of
    /// hashmap of strings along with the number of slurped values
    pub fn slurp<S: AsRef<str>>(
        self,
        args_slice: &[S],
    ) -> Result<(WeakOptionsDecoder, usize), OptionsSlurperError> {
        let mut slurped_args: HashMap<String, Vec<String>> = HashMap::new();
        let mut count = 0;

        let mut prev_arg: Option<&OptionDeclaration> = None;

        while let Some(arg) = args_slice.get(count).map(<S as AsRef<str>>::as_ref) {
            if let Some(option) = arg.strip_prefix("--") {
                // is an option of some sort

                if let Some(prev) = prev_arg {
                    // can't use -- as value
                    return Err(OptionsSlurperError::OptionTakesValueButNoneProvided(
                        prev.flag().to_string(),
                    ));
                }

                if let Some(declaration) = self.decs.get(option) {
                    if declaration.takes_value() {
                        prev_arg = Some(declaration);
                    } else {
                        slurped_args
                            .entry(option.to_string())
                            .or_default()
                            .push(String::new());
                    }
                } else {
                    return Err(OptionsSlurperError::UnknownOption(option.to_string()));
                }
            } else {
                // can either be a value for the previous flag or a different task
                match prev_arg {
                    Some(v) => {
                        let value = arg;
                        let option = v.flag().to_string();
                        if v.takes_value() {
                            slurped_args
                                .entry(option)
                                .or_default()
                                .push(value.to_string());
                            prev_arg = None;
                        } else {
                            break;
                        }
                    }
                    None => {
                        // A task has been found
                        break;
                    }
                }
            }

            count += 1;
        }

        if let Some(prev) = prev_arg {
            Err(OptionsSlurperError::OptionTakesValueButNoneProvided(
                prev.flag().to_string(),
            ))
        } else {
            ok!(self.decs.new_weak(slurped_args), count)
        }
    }
}

/// An error occurred while slurping task options
#[derive(Debug, thiserror::Error)]
pub enum OptionsSlurperError {
    #[error("No known option {0}")]
    UnknownOption(String),
    #[error("Given option {0} does not take a value")]
    OptionDoesNotTakeValue(String),
    #[error("Given option {0} takes a value but none provided")]
    OptionTakesValueButNoneProvided(String),
}

/// Provides the struct to decode options
#[derive(Clone, Debug)]
pub struct WeakOptionsDecoder {
    option_dec_string: String,
    fed_options: HashMap<String, Vec<String>>,
}

impl WeakOptionsDecoder {
    /// Try to upgrade this weak options decoder into an actual options decoder
    pub fn upgrade(self, decs: &OptionDeclarations) -> Result<OptionsDecoder, OptionsDecoderError> {
        if decs.task_type != self.option_dec_string {
            Err(OptionsDecoderError::OptionsMismatch)
        } else {
            Ok(OptionsDecoder {
                decs,
                fed_options: self.fed_options,
            })
        }
    }
}

/// Decodes a list of arbitrary arguments into a set of [`OptionRequest`s](OptionRequest).
///
/// Takes a set of
pub struct OptionsDecoder<'dec> {
    decs: &'dec OptionDeclarations,
    fed_options: HashMap<String, Vec<String>>,
}

pub type DecoderResult<T> = Result<T, OptionsDecoderError>;

impl<'dec> OptionsDecoder<'dec> {
    /// gets the declaration and verifies its correct
    fn get_option_dec(&self, flag: &str) -> DecoderResult<&OptionDeclaration> {
        let dec = self
            .decs
            .get(flag)
            .ok_or(OptionsDecoderError::InvalidOption(flag.to_string()))?;

        if dec.is_flag()
            && self
                .fed_options
                .get(flag)
                .map(|v| v != &flag_value_entry())
                .unwrap_or(false)
        {
            error!("flag has bad value: {:?}", self.fed_options.get(flag));
            return Err(OptionsDecoderError::OptionDoesNotTakeValue(
                flag.to_string(),
            ));
        }

        if dec.takes_value() {
            match self.fed_options.get(flag) {
                None => {
                    if !dec.optional() {
                        return Err(OptionsDecoderError::OptionNotOptional(flag.to_string()));
                    }
                }
                Some(v) => {
                    if v.len() > 1 && !dec.allow_multiple_values() {
                        return Err(OptionsDecoderError::OptionDoesNotTakeMultipleValue(
                            flag.to_string(),
                        ));
                    }
                }
            }
        }

        Ok(dec)
    }

    /// Check whether a flag is present
    pub fn flag_present(&self, flag: &str) -> DecoderResult<bool> {
        let dec = self.get_option_dec(flag)?;
        if dec.is_flag() {
            if let Some(entry) = self.fed_options.get(flag) {
                assert_eq!(entry, &flag_value_entry(), "flag improperly set in options");
                ok!(true)
            } else {
                ok!(false)
            }
        } else {
            Err(OptionsDecoderError::OptionNotFlag(flag.to_string()))
        }
    }

    /// Get a value for a flag, if present. Only returns Ok(None) if the option is optional, otherwise
    /// an Err() is returned.
    ///
    /// Will also return an error if multiple values are defined for this type.
    pub fn get_value<T: 'static>(&self, flag: &str) -> DecoderResult<Option<T>> {
        let declaration = self.get_option_dec(flag)?;
        if declaration.is_flag() {
            return Err(OptionsDecoderError::OptionDoesNotTakeValue(
                flag.to_string(),
            ));
        }

        if declaration.allow_multiple_values() {
            return Err(OptionsDecoderError::OptionTakesMultipleValue(
                flag.to_string(),
            ));
        }

        if declaration.flag_type != TypeId::of::<T>() {
            return Err(OptionsDecoderError::incorrect_type::<T>(flag));
        }

        if let Some(values) = self.fed_options.get(flag) {
            let value = values.first().unwrap();
            let parse_function = declaration.parse_value.as_ref().unwrap();
            let parsed: Box<dyn Any> = parse_function(value)?;
            Ok(Some(*parsed.downcast::<T>().unwrap()))
        } else {
            return if declaration.optional {
                Ok(None)
            } else {
                Err(OptionsDecoderError::OptionNotOptional(flag.to_string()))
            };
        }
    }

    /// Get all values for a flag, if present. Only returns Ok(None) if the option is optional, otherwise
    /// an Err() is returned.
    ///
    /// Will also return an error if this option does not accept multiple values.
    pub fn get_values<T: 'static>(&self, flag: &str) -> DecoderResult<Option<Vec<T>>> {
        let declaration = self.get_option_dec(flag)?;
        if declaration.is_flag() {
            return Err(OptionsDecoderError::OptionDoesNotTakeValue(
                flag.to_string(),
            ));
        }

        if !declaration.allow_multiple_values() {
            return Err(OptionsDecoderError::OptionDoesNotTakeMultipleValue(
                flag.to_string(),
            ));
        }

        if declaration.flag_type != TypeId::of::<T>() {
            return Err(OptionsDecoderError::incorrect_type::<T>(flag));
        }

        if let Some(values) = self.fed_options.get(flag) {
            let parse_function = declaration.parse_value.as_ref().unwrap();
            let output: Vec<T> = values
                .iter()
                .map(|value| {
                    let parsed: Box<dyn Any> = parse_function(value)?;
                    Ok(*parsed.downcast::<T>().unwrap())
                })
                .collect::<DecoderResult<Vec<_>>>()?;
            Ok(Some(output))
        } else {
            return if declaration.optional {
                Ok(None)
            } else {
                Err(OptionsDecoderError::OptionNotOptional(flag.to_string()))
            };
        }
    }
}

/// An error occurred while decoding task options
#[derive(Debug, thiserror::Error)]
pub enum OptionsDecoderError {
    #[error("Given options declarations not meant for this options decoder")]
    OptionsMismatch,
    #[error("Given option {0} is not a flag")]
    OptionNotFlag(String),
    #[error("Given option {0} requires a value to be provided")]
    OptionNotOptional(String),
    #[error("Given option {0} does not take a value")]
    OptionDoesNotTakeValue(String),
    #[error("Given option {0} does not take a value")]
    OptionDoesNotTakeMultipleValue(String),
    #[error("Given option {0} takes multiple values")]
    OptionTakesMultipleValue(String),
    #[error("Given string is not a registered option")]
    InvalidOption(String),
    #[error(transparent)]
    ValueParserError(#[from] Box<dyn Error + Send + Sync>),
    #[error("Given option {option} does not take values of type {given_type}")]
    IncorrectType { given_type: String, option: String },
}

impl OptionsDecoderError {
    pub fn incorrect_type<T>(option: &str) -> Self {
        Self::IncorrectType {
            given_type: type_name::<T>().to_string(),
            option: option.to_string(),
        }
    }
}

#[cfg(test)]
mod slurper_tests {
    use super::*;
    use crate::defaults::tasks::Empty;
    use more_collection_macros::map;

    #[test]
    fn slurp_flags() {
        let args = ["--flag1", "--flag2", "task"];
        let options = OptionDeclarations::new::<Empty, _>([
            OptionDeclarationBuilder::flag("flag1").build(),
            OptionDeclarationBuilder::flag("flag2").build(),
        ]);

        let slurper = OptionsSlurper::new(&options);
        let (map, slurped) = slurper.slurp(&args).unwrap();
        assert_eq!(slurped, 2, "only 2 values should be slurped");
        assert_eq!(
            map.fed_options,
            map![
                "flag1".to_string() => flag_value_entry(),
                "flag2".to_string() => flag_value_entry()
            ]
        );
    }

    #[test]
    fn slurp_values() {
        let args = ["--flag1", "value1", "--flag2", "value2", "task"];
        let options = OptionDeclarations::new::<Empty, _>([
            OptionDeclarationBuilder::<String>::new("flag1")
                .use_from_str()
                .build(),
            OptionDeclarationBuilder::<String>::new("flag2")
                .use_from_str()
                .build(),
        ]);

        let slurper = OptionsSlurper::new(&options);
        let (map, slurped) = slurper.slurp(&args).unwrap();
        assert_eq!(slurped, 4, "only 4 values should be slurped");
        assert_eq!(
            map.fed_options,
            map![
                "flag1".to_string() => vec!["value1".to_string()],
                "flag2".to_string() => vec!["value2".to_string()]
            ]
        );
    }

    #[test]
    fn mix_slurp_values_and_flags() {
        let args = ["--flag1", "value1", "--flag3", "--flag2", "value2", "task"];
        let options = OptionDeclarations::new::<Empty, _>([
            OptionDeclarationBuilder::<String>::new("flag1")
                .use_from_str()
                .build(),
            OptionDeclarationBuilder::<String>::new("flag2")
                .use_from_str()
                .build(),
            OptionDeclarationBuilder::flag("flag3").build(),
        ]);

        let slurper = OptionsSlurper::new(&options);
        let (map, slurped) = slurper.slurp(&args).unwrap();
        assert_eq!(slurped, 5, "only 5 values should be slurped");
        assert_eq!(
            map.fed_options,
            map![
                "flag1".to_string() => vec!["value1".to_string()],
                "flag2".to_string() => vec!["value2".to_string()],
                "flag3".to_string() => flag_value_entry()
            ]
        );
    }

    #[test]
    fn slurp_multiple_values() {
        let args = ["--flag1", "value1", "--flag1", "value2"];
        let options =
            OptionDeclarations::new::<Empty, _>([OptionDeclarationBuilder::<String>::new("flag1")
                .use_from_str()
                .allow_multiple_values(true)
                .build()]);

        let slurper = OptionsSlurper::new(&options);
        let (map, slurped) = slurper.slurp(&args).unwrap();
        assert_eq!(slurped, 4, "only 4 values should be slurped");
        assert_eq!(
            map.fed_options,
            map![
                "flag1".to_string() => vec!["value1".to_string(), "value2".to_string()],
            ]
        );
    }

    #[test]
    fn flag_not_a_value() {
        let args = ["--flag1", "--flag2", "task"];
        let options = OptionDeclarations::new::<Empty, _>([
            OptionDeclarationBuilder::<String>::new("flag1")
                .use_from_str()
                .build(),
            OptionDeclarationBuilder::flag("flag2").build(),
        ]);

        let slurper = OptionsSlurper::new(&options);
        assert!(slurper.slurp(&args).is_err());
    }

    #[test]
    fn option_missing_value() {
        let args = ["--flag1"];
        let options = OptionDeclarations::new::<Empty, _>([
            OptionDeclarationBuilder::<String>::new("flag1")
                .use_from_str()
                .build(),
            OptionDeclarationBuilder::flag("flag2").build(),
        ]);

        let slurper = OptionsSlurper::new(&options);
        assert!(slurper.slurp(&args).is_err());
    }
}

#[cfg(test)]
mod decoder_tests {
    use super::*;
    use crate::defaults::tasks::Empty;

    #[test]
    fn can_use_value() {
        let options =
            OptionDeclarations::new::<Empty, _>([OptionDeclarationBuilder::<i32>::new("count")
                .use_from_str()
                .build()]);

        let slurper = options.slurper();
        let (weak, _) = slurper.slurp(&["--count", "15"]).unwrap();

        let upgraded = weak.upgrade(&options).unwrap();

        let count = upgraded.get_value::<i32>("count").unwrap();
        assert_eq!(count, Some(15));
    }

    #[test]
    fn optional_not_required() {
        let options =
            OptionDeclarations::new::<Empty, _>([OptionDeclarationBuilder::<i32>::new("count")
                .use_from_str()
                .optional(true)
                .build()]);

        let slurper = options.slurper();
        let (weak, _) = slurper.slurp::<&str>(&[]).unwrap();

        let upgraded = weak.upgrade(&options).unwrap();

        let count = upgraded.get_value::<i32>("count").unwrap();
        assert_eq!(count, None);
    }

    #[test]
    fn can_use_multiple_values() {
        let options =
            OptionDeclarations::new::<Empty, _>([OptionDeclarationBuilder::<i32>::new("count")
                .use_from_str()
                .allow_multiple_values(true)
                .build()]);

        let slurper = options.slurper();
        let (weak, _) = slurper.slurp(&["--count", "15", "--count", "16"]).unwrap();

        let upgraded = weak.upgrade(&options).unwrap();

        let count = upgraded.get_values::<i32>("count").unwrap();
        assert_eq!(count, Some(vec![15, 16]));
    }

    #[test]
    fn optional_not_required_multiples() {
        let options =
            OptionDeclarations::new::<Empty, _>([OptionDeclarationBuilder::<i32>::new("count")
                .use_from_str()
                .optional(true)
                .allow_multiple_values(true)
                .build()]);

        let slurper = options.slurper();
        let (weak, _) = slurper.slurp::<&str>(&[]).unwrap();

        let upgraded = weak.upgrade(&options).unwrap();

        let count = upgraded.get_values::<i32>("count").unwrap();
        assert_eq!(count, None);
    }

    #[test]
    fn slurp_multiple_values_can_cause_errors_if_invalid() {
        let args = ["--flag1", "value1", "--flag1", "value2"];
        let options =
            OptionDeclarations::new::<Empty, _>([OptionDeclarationBuilder::<String>::new("flag1")
                .use_from_str()
                .takes_value(true)
                .allow_multiple_values(false)
                .build()]);

        let slurper = OptionsSlurper::new(&options);
        let (weak, _) = slurper.slurp(&args).unwrap();

        let upgraded = weak.upgrade(&options).unwrap();

        let err = upgraded.get_value::<String>("flag1");
        if let Err(OptionsDecoderError::OptionDoesNotTakeMultipleValue(_)) = err {
        } else {
            panic!("Should cause an error, does take multiple values error")
        }
    }

    #[test]
    fn non_optional_cause_errors_if_missing() {
        let args = ["--flag1", "value1"];
        let options = OptionDeclarations::new::<Empty, _>([
            OptionDeclarationBuilder::<String>::new("flag1")
                .use_from_str()
                .build(),
            OptionDeclarationBuilder::<String>::new("flag2")
                .use_from_str()
                .build(),
        ]);

        let slurper = OptionsSlurper::new(&options);
        let (weak, _) = slurper.slurp(&args).unwrap();

        let upgraded = weak.upgrade(&options).unwrap();

        let err = upgraded.get_value::<String>("flag2");
        if let Err(OptionsDecoderError::OptionNotOptional(_)) = err {
        } else {
            panic!("Should cause an error, flag2 is not optional")
        }
    }
}
