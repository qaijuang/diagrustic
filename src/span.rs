use core::range::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileId(usize);

impl FileId {
    pub(crate) fn new(id: usize) -> Self {
        Self(id)
    }
    #[must_use]
    pub fn index(self) -> usize {
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
    pub(crate) fn new(file_id: FileId, range: Range<usize>) -> Self {
        Self { file_id, range }
    }

    #[must_use]
    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    #[must_use]
    pub fn start(&self) -> usize {
        self.range.start
    }

    #[must_use]
    pub fn end(&self) -> usize {
        self.range.end
    }

    #[allow(clippy::len_without_is_empty)]
    #[must_use]
    pub fn len(&self) -> usize {
        self.range.iter().len()
    }

    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range
    }
}
