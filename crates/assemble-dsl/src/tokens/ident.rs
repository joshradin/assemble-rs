use crate::source::Source;
use crate::span::Span;
use crate::tokens::Token;

/// An identifier
#[derive(Debug)]
pub struct Ident {
    span: Span
}

impl Token for Ident {
    type Err = ();

    fn span(&self) -> Span {
        self.span.clone()
    }

    fn parse(source: &Source, index: usize) -> Result<Self, Self::Err> {
        let result = source.text()?;


    }
}