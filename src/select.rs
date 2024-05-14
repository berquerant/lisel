use crate::index::Type;
use crate::lineparse::range;
use crate::str::rstrip;
use log::debug;
use std::cmp::PartialEq;
use std::io::BufRead;
use std::iter::Iterator;
use thiserror;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SelectError {
    #[error("IO ({0})")]
    Io(String),
    #[error("Parse ({0})")]
    Parse(String),
}

pub struct Select<T, I>
where
    T: BufRead,
    I: BufRead,
{
    index_type: Option<Type>,
    invert_match: bool,

    target_stream: T,
    target_stream_linum: u32,
    index_stream: I,
    index_stream_linum: u32,
    /// End of iterator.
    eoi: bool,
}

impl<T, I> Iterator for Select<T, I>
where
    T: BufRead,
    I: BufRead,
{
    type Item = Result<String, SelectError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eoi {
            return None;
        }

        self.target_stream_linum += 1;
        debug!("Target|line={}", self.target_stream_linum);
        let mut line = String::new();
        match self.target_stream.read_line(&mut line) {
            Err(x) => {
                self.disable();
                Some(Err(SelectError::Io(x.to_string())))
            }
            // EOF of target
            Ok(0) => {
                self.disable();
                self.next()
            }
            Ok(_) => match self.select(self.target_stream_linum) {
                SelectResult::Error(x) => {
                    self.disable();
                    Some(Err(x))
                }
                // EOF of index
                SelectResult::EndOfIndex => {
                    self.disable();
                    self.next()
                }
                SelectResult::Accept => Some(Ok(line)),
                SelectResult::Deny => self.next(),
            },
        }
    }
}

#[derive(Debug, PartialEq)]
enum SelectResult {
    Error(SelectError),
    EndOfIndex,
    Accept,
    Deny,
}

impl<T, I> Select<T, I>
where
    T: BufRead,
    I: BufRead,
{
    pub fn new(
        target_stream: T,
        index_stream: I,
        index_type: Option<Type>,
        invert_match: bool,
    ) -> Select<T, I> {
        Select {
            index_type,
            invert_match,
            target_stream,
            index_stream,
            target_stream_linum: 0,
            eoi: false,
            index_stream_linum: 0,
        }
    }

    /// Disable self as an iterator.
    fn disable(&mut self) {
        self.eoi = true;
    }

    fn select(&mut self, linum: u32) -> SelectResult {
        match &self.index_type {
            Some(r @ Type::Re(_)) => {
                let mut index_line = String::new();
                self.index_stream_linum += 1;
                let s = self.index_stream.read_line(&mut index_line);
                debug!(
                    "Re|target={}|index={}|line={}",
                    linum, self.index_stream_linum, index_line
                );
                rstrip(&mut index_line);
                match s {
                    Err(x) => SelectResult::Error(SelectError::Io(x.to_string())),
                    // invert end of index, accept all lines
                    Ok(0) if self.invert_match => SelectResult::Accept,
                    // ignore lines in the index file that exceed the number of lines in the target file
                    Ok(0) => SelectResult::EndOfIndex,
                    Ok(_) if r.select(0, &index_line) != self.invert_match => SelectResult::Accept,
                    Ok(_) => SelectResult::Deny,
                }
            }
            // since we have passed the specified range, we will find a new expression
            Some(r @ Type::Number(_)) if r.end() < linum => {
                self.index_type = None;
                self.select(linum)
            }
            Some(r @ Type::Number(_)) if r.select(linum, "") != self.invert_match => {
                SelectResult::Accept
            }
            Some(Type::Number(_)) => SelectResult::Deny,
            None => {
                let mut index_line = String::new();
                self.index_stream_linum += 1;
                let s = self.index_stream.read_line(&mut index_line);
                rstrip(&mut index_line);
                debug!(
                    "Number|target={}|index={}|line={}",
                    linum, self.index_stream_linum, index_line
                );
                match s {
                    Err(x) => SelectResult::Error(SelectError::Io(x.to_string())),
                    // invert end of index, accept all lines
                    Ok(0) if self.invert_match => SelectResult::Accept,
                    // ignore lines in the index file that exceed the number of lines in the target file
                    Ok(0) => SelectResult::EndOfIndex,
                    // ignore empty lines
                    Ok(_) if index_line.is_empty() => self.select(linum),
                    Ok(_) => match range(&index_line) {
                        Err(x) => SelectResult::Error(SelectError::Parse(format!(
                            "Number|target={}|index={}|line={}|result={}",
                            linum, self.index_stream_linum, &index_line, x
                        ))),
                        Ok((_, x)) => {
                            debug!(
                                "Parsed|target={}|index={}|line={}|range={:?}",
                                linum, self.index_stream_linum, &index_line, x
                            );
                            self.index_type = Some(Type::Number(x));
                            self.select(linum)
                        }
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    use std::io::BufReader;

    macro_rules! test_select_lines {
        ($name:ident, $target:expr, $index:expr, $index_type:expr, $invert_match:expr, $want:expr) => {
            #[test]
            fn $name() {
                let target = BufReader::new($target.as_bytes());
                let index = BufReader::new($index.as_bytes());
                let s = Select::new(target, index, $index_type, $invert_match);
                let got: Vec<String> = s.map(|x| x.unwrap()).collect();
                assert_eq!($want, got);
            }
        };
    }

    test_select_lines!(
        select_lines_number_single,
        "l1\nl2\nl3\nl4\nl5\n",
        "1\n3\n",
        None,
        false,
        vec!["l1\n", "l3\n"]
    );
    test_select_lines!(
        select_lines_number_range,
        "l1\nl2\nl3\nl4\nl5\n",
        "2,4\n",
        None,
        false,
        vec!["l2\n", "l3\n", "l4\n"]
    );
    test_select_lines!(
        select_lines_number_ranges,
        "l1\nl2\nl3\nl4\nl5\n",
        "1,1\n3,4\n",
        None,
        false,
        vec!["l1\n", "l3\n", "l4\n"]
    );
    test_select_lines!(
        select_lines_number_ranges_invert,
        "l1\nl2\nl3\nl4\nl5\n",
        "1,1\n3,4\n",
        None,
        true,
        vec!["l2\n", "l5\n"]
    );

    test_select_lines!(
        select_lines_re,
        "l1\nl2\nl3\n",
        "1\n\n1\n",
        Some(Type::Re(Regex::new(".+").unwrap())),
        false,
        vec!["l1\n", "l3\n"]
    );
    test_select_lines!(
        select_lines_re_smaller_target,
        "l1\nl2\nl3\n",
        "1\n\n1\n1\n",
        Some(Type::Re(Regex::new(".+").unwrap())),
        false,
        vec!["l1\n", "l3\n"]
    );
    test_select_lines!(
        select_lines_re_smaller_target_invert,
        "l1\nl2\nl3\n",
        "1\n\n1\n1\n",
        Some(Type::Re(Regex::new(".+").unwrap())),
        true,
        vec!["l2\n"]
    );
    test_select_lines!(
        select_lines_re_smaller_index,
        "l1\nl2\nl3\n",
        "1\n",
        Some(Type::Re(Regex::new(".+").unwrap())),
        false,
        vec!["l1\n"]
    );
    test_select_lines!(
        select_lines_re_smaller_index_invert,
        "l1\nl2\nl3\n",
        "1\n",
        Some(Type::Re(Regex::new(".+").unwrap())),
        true,
        vec!["l2\n", "l3\n"]
    );

    macro_rules! test_select {
        ($name:ident, $index:expr, $index_type:expr, $linum:expr, $want:expr, $want_inverse:expr) => {
            #[test]
            fn $name() {
                let index = $index.clone();
                let inverse_index = BufReader::new(index.as_bytes());
                let inverse_index_type = $index_type.clone();

                let mut s = Select::new(
                    BufReader::new("".as_bytes()),
                    BufReader::new(index.as_bytes()),
                    $index_type,
                    false,
                );
                let got = s.select($linum);
                assert_eq!($want, got, "want {:?} got {:?}", $want, got);

                let mut s = Select::new(
                    BufReader::new("".as_bytes()),
                    inverse_index,
                    inverse_index_type,
                    true,
                );
                let got = s.select($linum);
                assert_eq!(
                    $want_inverse, got,
                    "invert want {:?} got {:?}",
                    $want_inverse, got
                );
            }
        };
    }

    test_select!(
        select_number_single,
        "1\n",
        None,
        1,
        SelectResult::Accept,
        SelectResult::Deny
    );
    test_select!(
        select_number_single_not_matched,
        "1\n",
        None,
        2,
        SelectResult::EndOfIndex,
        SelectResult::Accept
    );
    test_select!(
        select_number_interval_matched,
        "1,3\n",
        None,
        2,
        SelectResult::Accept,
        SelectResult::Deny
    );
    test_select!(
        select_number_single_matched_update_index_from_none,
        "1\n2\n",
        None,
        2,
        SelectResult::Accept,
        SelectResult::Deny
    );
    test_select!(
        select_number_single_matched_update_index_from_single,
        "2\n",
        None,
        2,
        SelectResult::Accept,
        SelectResult::Deny
    );
    test_select!(
        select_number_interval_matched_update_index_from_interval,
        "5,6\n",
        None,
        5,
        SelectResult::Accept,
        SelectResult::Deny
    );
    test_select!(
        select_number_interval_not_matched_update_index_from_interval,
        "5,6\n",
        None,
        7,
        SelectResult::EndOfIndex,
        SelectResult::Accept
    );

    test_select!(
        select_re,
        "1\n",
        Some(Type::Re(Regex::new(".+").unwrap())),
        10, // ignored
        SelectResult::Accept,
        SelectResult::Deny
    );
    test_select!(
        select_eoi,
        "",
        Some(Type::Re(Regex::new(".+").unwrap())),
        10, // ignored
        SelectResult::EndOfIndex,
        SelectResult::Accept
    );
}
