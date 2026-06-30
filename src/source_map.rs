use alloc::alloc::{Allocator, Global};
use alloc::vec::Vec;

use crate::acow::{Acow, IntoAcow};
use crate::span::{ByteSpan, FileId, Span};

pub struct Source<'map, 'alloc, A: Allocator> {
    source_map: &'map SourceMap<'alloc, A>,
    file_id: FileId,
}

impl<A: Allocator> Copy for Source<'_, '_, A> {}
impl<A: Allocator> Clone for Source<'_, '_, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'map, 'alloc, A: Allocator> Source<'map, 'alloc, A> {
    const fn new(source_map: &'map SourceMap<'alloc, A>, file_id: FileId) -> Self {
        Self { source_map, file_id }
    }

    #[must_use]
    pub fn span(&self, span: impl Into<ByteSpan>) -> Span {
        self.source_map.span(self.file_id, span.into())
    }

    /// # Panics
    ///
    /// Panics only if this handle no longer refers to a file inside its source map.
    #[must_use]
    pub fn text(&self) -> &str {
        self.source_map.source(self.file_id).expect("source handle should reference a file")
    }

    /// # Panics
    ///
    /// Panics only if this handle no longer refers to a file inside its source map.
    #[must_use]
    pub fn filename(&self) -> &str {
        self.source_map.filename(self.file_id).expect("source handle should reference a file")
    }

    #[must_use]
    pub const fn source_map(&self) -> &'map SourceMap<'alloc, A> {
        self.source_map
    }
}

#[derive(Debug)]
pub struct SourceFile<'alloc, A: Allocator> {
    pub name: Acow<'alloc, A>,
    pub source: Acow<'alloc, A>,
    line_starts: Vec<usize, &'alloc A>,
}

#[derive(Debug)]
pub struct SourceMap<'alloc, A: Allocator> {
    files: Vec<SourceFile<'alloc, A>, &'alloc A>,
    alloc: &'alloc A,
}

impl Default for SourceMap<'_, Global> {
    fn default() -> Self {
        Self::new_in(&Global)
    }
}

impl<'alloc, A: Allocator> SourceMap<'alloc, A> {
    #[must_use]
    pub const fn new_in(alloc: &'alloc A) -> Self {
        Self { files: Vec::new_in(alloc), alloc }
    }

    /// Add a source file, returning its `FileId`.
    pub fn add_file(
        &mut self,
        name: impl IntoAcow<'alloc, A>,
        source: impl IntoAcow<'alloc, A>,
    ) -> FileId {
        let id = FileId::new(self.files.len());
        let name = name.into_acow(self.alloc);
        let source = source.into_acow(self.alloc);
        let line_starts = self.line_starts(source.as_str());
        self.files.push(SourceFile { name, source, line_starts });
        id
    }

    #[must_use]
    pub fn add_source(
        &mut self,
        name: impl IntoAcow<'alloc, A>,
        source: impl IntoAcow<'alloc, A>,
    ) -> Source<'_, 'alloc, A> {
        let file_id = self.add_file(name, source);
        Source::new(self, file_id)
    }

    #[must_use]
    pub fn source_handle(&self, file_id: FileId) -> Option<Source<'_, 'alloc, A>> {
        self.files.get(file_id.index())?;
        Some(Source::new(self, file_id))
    }

    /// Create a Span for a given file and byte range.
    /// Panics in debug if range is out of bounds.
    #[must_use]
    pub fn span(&self, file_id: FileId, range: impl Into<ByteSpan>) -> Span {
        let range = range.into();
        debug_assert!(file_id.index() < self.files.len(), "file_id out of bounds");
        let source = &self.files[file_id.index()].source;
        debug_assert!(range.end() <= source.len(), "range out of bounds");
        debug_assert!(
            source.is_char_boundary(range.start()) && source.is_char_boundary(range.end()),
            "range must be at character boundaries"
        );
        Span::new(file_id, range)
    }

    /// Access the source text for a file.
    #[must_use]
    pub fn source(&self, file_id: FileId) -> Option<&str> {
        self.files.get(file_id.index()).map(|f| f.source.as_str())
    }

    /// Access the file name.
    #[must_use]
    pub fn filename(&self, file_id: FileId) -> Option<&str> {
        self.files.get(file_id.index()).map(|f| f.name.as_str())
    }

    /// Convert a byte offset to line and column (1‑based).
    #[must_use]
    pub fn line_col(&self, file_id: FileId, offset: usize) -> Option<(usize, usize)> {
        let file = self.files.get(file_id.index())?;
        if offset > file.source.len() || !file.source.as_str().is_char_boundary(offset) {
            return None;
        }
        let line_idx = match file.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        let line_start = file.line_starts[line_idx];
        let column = file.source.as_str()[line_start..offset].chars().count() + 1;
        Some((line_idx + 1, column))
    }

    /// Get a line of source by line number (1‑based).
    #[must_use]
    pub fn line(&self, file_id: FileId, line_number: usize) -> Option<&str> {
        if line_number == 0 {
            return None;
        }
        let file = self.files.get(file_id.index())?;
        let start = *file.line_starts.get(line_number - 1)?;
        let end = file
            .line_starts
            .get(line_number)
            .map_or(file.source.len(), |next_start| next_start.saturating_sub(1));
        Some(&file.source.as_str()[start..end])
    }

    /// Return the start of the line containing `offset`.
    #[must_use]
    pub fn line_start(&self, file_id: FileId, offset: usize) -> Option<usize> {
        let file = self.files.get(file_id.index())?;
        if offset > file.source.len() {
            return None;
        }
        let line_idx = match file.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        Some(file.line_starts[line_idx])
    }

    #[must_use]
    pub(crate) fn line_start_for_line(&self, file_id: FileId, line_number: usize) -> Option<usize> {
        if line_number == 0 {
            return None;
        }
        let file = self.files.get(file_id.index())?;
        file.line_starts.get(line_number - 1).copied()
    }

    fn line_starts(&self, source: &str) -> Vec<usize, &'alloc A> {
        let mut starts = Vec::new_in(self.alloc);
        starts.push(0);
        starts.extend(source.match_indices('\n').map(|(idx, _)| idx + 1));
        starts
    }
}

#[cfg(test)]
mod tests {
    use super::SourceMap;

    #[test]
    fn add_source_returns_file_scoped_span_handle() {
        let mut source_map = SourceMap::default();
        let source = source_map.add_source("input.rs", "let value = 1;");

        let span = source.span(4..9);

        assert_eq!(source.filename(), "input.rs");
        assert_eq!(source.text(), "let value = 1;");
        assert_eq!(span.start(), 4);
        assert_eq!(span.end(), 9);
        assert_eq!(span.range(), 4..9);
        assert_eq!(source_map.filename(span.file_id()), Some("input.rs"));
    }

    #[test]
    fn line_col_handles_multiline_offsets() {
        let mut source_map = SourceMap::default();
        let file = source_map.add_file("foo.rs", "alpha\nbeta\ngamma");

        assert_eq!(source_map.line_col(file, 0), Some((1, 1)));
        assert_eq!(source_map.line_col(file, 5), Some((1, 6)));
        assert_eq!(source_map.line_col(file, 6), Some((2, 1)));
        assert_eq!(source_map.line_col(file, 10), Some((2, 5)));
        assert_eq!(source_map.line_col(file, 11), Some((3, 1)));
        assert_eq!(source_map.line_col(file, 16), Some((3, 6)));
        assert_eq!(source_map.line_col(file, 17), None);
    }

    #[test]
    fn line_col_counts_characters_not_bytes() {
        let source = "let\tπ = \"中\";";
        let mut source_map = SourceMap::default();
        let file = source_map.add_file("unicode.rs", source);

        assert_eq!(source_map.line_col(file, source.find('π').unwrap()), Some((1, 5)));
        assert_eq!(source_map.line_col(file, source.find('中').unwrap()), Some((1, 10)));
    }

    #[test]
    fn line_and_line_start_use_one_based_lines() {
        let mut source_map = SourceMap::default();
        let file = source_map.add_file("foo.rs", "alpha\nbeta\ngamma");

        assert_eq!(source_map.line(file, 1), Some("alpha"));
        assert_eq!(source_map.line(file, 2), Some("beta"));
        assert_eq!(source_map.line(file, 3), Some("gamma"));
        assert_eq!(source_map.line(file, 4), None);
        assert_eq!(source_map.line_start(file, 0), Some(0));
        assert_eq!(source_map.line_start(file, 6), Some(6));
        assert_eq!(source_map.line_start(file, 16), Some(11));
        assert_eq!(source_map.line_start_for_line(file, 1), Some(0));
        assert_eq!(source_map.line_start_for_line(file, 2), Some(6));
        assert_eq!(source_map.line_start_for_line(file, 3), Some(11));
        assert_eq!(source_map.line_start_for_line(file, 4), None);
    }
}
