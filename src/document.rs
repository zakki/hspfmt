use std::borrow::Cow;

use crate::encoding::ResolvedEncoding;
use crate::lexer::lex_resolved;
use crate::parser::Item;
use crate::{Error, Kind};

/// Lossless tokens shared by every formatting pass. Unchanged bytes borrow the input.
#[derive(Debug, Clone)]
pub(crate) struct Atom<'a> {
    pub kind: Kind,
    pub text: Cow<'a, [u8]>,
    pub offset: usize,
}

impl<'a> Atom<'a> {
    pub fn slice(&self, begin: usize, end: usize) -> Cow<'a, [u8]> {
        match &self.text {
            Cow::Borrowed(text) => Cow::Borrowed(&text[begin..end]),
            Cow::Owned(text) => Cow::Owned(text[begin..end].to_vec()),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Document<'a> {
    pub atoms: Vec<Atom<'a>>,
}

impl<'a> Document<'a> {
    pub fn parse(source: &'a [u8], encoding: ResolvedEncoding) -> Result<Self, Error> {
        Ok(Self {
            atoms: lex_resolved(source, encoding)?
                .into_iter()
                .map(|token| Atom {
                    kind: token.kind,
                    text: Cow::Borrowed(&source[token.begin..token.end]),
                    offset: token.begin,
                })
                .collect(),
        })
    }

    pub fn text(&self, index: usize) -> &[u8] {
        &self.atoms[index].text
    }

    pub fn push(&mut self, kind: Kind, text: Cow<'a, [u8]>, offset: usize) {
        if !text.is_empty() {
            self.atoms.push(Atom { kind, text, offset });
        }
    }

    pub fn append(&mut self, atoms: &[Atom<'a>]) {
        self.atoms.extend_from_slice(atoms);
    }

    pub fn append_items(&mut self, items: &[Item<'a>], offset: usize) {
        for item in items {
            self.push(Kind::Space, item.gap.clone(), offset);
            self.push(item.kind, item.text.clone(), offset);
        }
    }

    pub fn byte_len(&self) -> usize {
        self.atoms.iter().map(|atom| atom.text.len()).sum()
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.range_bytes(0, self.atoms.len())
    }

    pub fn range_bytes(&self, begin: usize, end: usize) -> Vec<u8> {
        let mut bytes = Vec::new();
        for atom in &self.atoms[begin..end] {
            bytes.extend_from_slice(&atom.text);
        }
        bytes
    }

    pub fn items(&self, begin: usize, end: usize) -> Vec<Item<'a>> {
        let mut items = Vec::new();
        let mut gap: Cow<'a, [u8]> = Cow::Borrowed(b"");
        for atom in &self.atoms[begin..end] {
            if atom.kind == Kind::Space {
                if gap.is_empty() {
                    gap = atom.text.clone();
                } else {
                    gap.to_mut().extend_from_slice(&atom.text);
                }
            } else if atom.kind != Kind::Bom {
                items.push(Item {
                    kind: atom.kind,
                    text: atom.text.clone(),
                    gap,
                });
                gap = Cow::Borrowed(b"");
            }
        }
        items
    }
}
