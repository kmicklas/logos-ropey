use std::ops::Range;

/// A [`logos::Source`] which wraps a [`ropey::RopeSlice`].
///
/// To use it, set the `source` attribute on your `logos` derive to
/// `RopeSliceSource<'s>`:
///
/// ```rust
/// # use logos::Logos;
/// # use logos_ropey::RopeSliceSource;
/// #[derive(Logos)]
/// #[logos(source = RopeSliceSource<'s>)]
/// enum Token {
///     #[regex(".")]
///     Token,
/// }
/// ```
#[derive(ref_cast::RefCast, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct RopeSliceSource<'s>(pub ropey::RopeSlice<'s>);

impl<'s> std::fmt::Display for RopeSliceSource<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'s> From<ropey::RopeSlice<'s>> for RopeSliceSource<'s> {
    fn from(value: ropey::RopeSlice<'s>) -> Self {
        Self(value)
    }
}

impl<'s> From<&'s ropey::Rope> for RopeSliceSource<'s> {
    fn from(value: &'s ropey::Rope) -> Self {
        Self(value.slice(..))
    }
}

impl<'s> logos::Source for RopeSliceSource<'s> {
    type Slice<'a> = ropey::RopeSlice<'a> where 's: 'a;

    fn len(&self) -> usize {
        self.0.len_bytes()
    }

    fn read<'a, Chunk>(&'a self, offset: usize) -> Option<Chunk>
    where
        Chunk: logos::source::Chunk<'a>,
    {
        let (mut chunks, start, _, _) = self.0.chunks_at_byte(offset);
        let data = &chunks.next()?.as_bytes()[offset - start..];

        if data.len() < Chunk::SIZE {
            None
        } else {
            Some(unsafe { Chunk::from_ptr(data.as_ptr()) })
        }
    }

    unsafe fn read_unchecked<'a, Chunk>(&'a self, offset: usize) -> Chunk
    where
        Chunk: logos::source::Chunk<'a>,
    {
        self.read(offset).unwrap_unchecked()
    }

    fn slice(&self, range: Range<usize>) -> Option<Self::Slice<'_>> {
        self.0.get_byte_slice(range)
    }

    unsafe fn slice_unchecked(&self, range: Range<usize>) -> Self::Slice<'_> {
        self.slice(range).unwrap_unchecked()
    }

    fn find_boundary(&self, index: usize) -> usize {
        let c = self.0.byte_to_char(index);

        if index == self.0.char_to_byte(c) {
            index
        } else {
            self.0.char_to_byte(c + 1)
        }
    }

    fn is_boundary(&self, index: usize) -> bool {
        self.0
            .try_byte_to_char(index)
            .ok()
            .map(|c| self.0.char_to_byte(c))
            == Some(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(logos::Logos, Debug, PartialEq)]
    #[logos(source = RopeSliceSource<'s>)]
    enum Token {
        #[regex(r"[^,]*,")]
        Token,
    }

    #[test]
    fn test_source() {
        let mut rope = ropey::Rope::new();

        // Build a sufficiently large string that we exercise chunking.
        for len in 1..=1_000 {
            let mut token = str::repeat("x", len);
            token.push_str(",");
            rope.append(token.into());
        }

        // Make sure we have chunks.
        assert!(rope.chunks().count() > 10);

        let source = RopeSliceSource(rope.slice(..));
        let lexer = logos::Lexer::new(&source);

        assert_eq!(
            lexer
                .inspect(|t| assert_eq!(t.as_ref().ok(), Some(&Token::Token)))
                .count(),
            1_000
        );
    }
}
