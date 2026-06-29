use core::range::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileId(usize);

impl FileId {
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// A byte-range span inside a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    file_id: FileId,
    range: Range<usize>,
}

impl Span {
    /// Only the `SourceMap` may create spans.
    #[must_use]
    pub fn new(file_id: FileId, range: core::ops::Range<usize>) -> Self {
        Self { file_id, range: range.into() }
    }

    #[must_use]
    pub const fn file_id(&self) -> FileId {
        self.file_id
    }

    #[must_use]
    pub const fn start(&self) -> usize {
        self.range.start
    }

    #[must_use]
    pub const fn end(&self) -> usize {
        self.range.end
    }

    #[allow(clippy::len_without_is_empty)]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.range.end - self.range.start
    }

    #[must_use]
    pub const fn range(&self) -> Range<usize> {
        self.range
    }
}
