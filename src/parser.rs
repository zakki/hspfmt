use crate::util::{lower, one_of};
use crate::{Error, Kind, Options};
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub(crate) struct Item<'a> {
    pub(crate) kind: Kind,
    pub(crate) text: Cow<'a, [u8]>,
    pub(crate) gap: Cow<'a, [u8]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Frame {
    pub(crate) close: Vec<u8>,
    pub(crate) case_body: bool,
    pub(crate) line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct State {
    pub(crate) blocks: Vec<Frame>,
    pub(crate) function: bool,
    pub(crate) label: bool,
    pub(crate) chsp_module: Option<usize>,
    pub(crate) chsp_function: Option<usize>,
}

impl State {
    pub(crate) fn same_structure(&self, other: &State) -> bool {
        if self.blocks.len() != other.blocks.len() {
            return false;
        }
        for (a, b) in self.blocks.iter().zip(other.blocks.iter()) {
            if a.close != b.close || a.case_body != b.case_body {
                return false;
            }
        }
        self.chsp_module.is_some() == other.chsp_module.is_some()
            && self.chsp_function.is_some() == other.chsp_function.is_some()
    }

    pub(crate) fn depth(&self, options: &Options) -> usize {
        let mut n = options
            .base_indent
            .max(usize::from(self.function || self.label));
        for frame in &self.blocks {
            n += if one_of(&frame.close, &[b"loop", b"wend", b"next", b"until"]) {
                options.loop_indent
            } else if frame.case_body {
                2
            } else {
                1
            };
        }
        n
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Conditional {
    pub(crate) before: State,
    pub(crate) branch: Option<State>,
    pub(crate) has_else: bool,
    pub(crate) line: usize,
}

pub(crate) fn close_block(state: &mut State, close: &[u8], line: usize) -> Result<(), Error> {
    if state.blocks.is_empty() || state.blocks.last().unwrap().close != close {
        return Err(Error::new(
            Some(line),
            format!(
                "unmatched block terminator: {}",
                String::from_utf8_lossy(close)
            ),
        ));
    }
    state.blocks.pop();
    Ok(())
}

pub(crate) fn parse_line(items: &[Item], state: &mut State, line: usize) -> Result<(), Error> {
    let mut statement = true;
    for item in items {
        if item.kind == Kind::Comment {
            continue;
        }
        let word = lower(&item.text);
        if item.text == b"{".as_slice() {
            state.blocks.push(Frame {
                close: b"}".to_vec(),
                case_body: false,
                line,
            });
            statement = true;
        } else if item.text == b"}".as_slice() {
            close_block(state, b"}", line)?;
            statement = true;
        } else if item.text == b":".as_slice() {
            statement = true;
        } else {
            if statement {
                if word == b"repeat" || word == b"foreach" {
                    state.blocks.push(Frame {
                        close: b"loop".to_vec(),
                        case_body: false,
                        line,
                    });
                } else if word == b"while" {
                    state.blocks.push(Frame {
                        close: b"wend".to_vec(),
                        case_body: false,
                        line,
                    });
                } else if word == b"for" {
                    state.blocks.push(Frame {
                        close: b"next".to_vec(),
                        case_body: false,
                        line,
                    });
                } else if word == b"do" {
                    state.blocks.push(Frame {
                        close: b"until".to_vec(),
                        case_body: false,
                        line,
                    });
                } else if word == b"switch" {
                    state.blocks.push(Frame {
                        close: b"swend".to_vec(),
                        case_body: false,
                        line,
                    });
                } else if one_of(&word, &[b"loop", b"wend", b"next", b"until", b"swend"]) {
                    close_block(state, &word, line)?;
                } else if word == b"case" || word == b"default" {
                    if state.blocks.is_empty() || state.blocks.last().unwrap().close != b"swend" {
                        return Err(Error::new(Some(line), "case/default outside switch"));
                    }
                    state.blocks.last_mut().unwrap().case_body = true;
                }
            }
            statement = false;
        }
    }
    Ok(())
}
