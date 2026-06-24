use alloc::alloc::{Allocator, Global};
use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::ops::Range;

use crate::span::{FileId, Span};

#[derive(Debug)]
pub struct SourceFile<'alloc, A: Allocator> {
    pub name: Cow<'static, str>,
    pub source: Cow<'static, str>,
    line_starts: Vec<usize, &'alloc A>,
}

#[derive(Debug)]
pub struct SourceMap<'alloc, A: Allocator = Global> {
    files: Vec<SourceFile<'alloc, A>, &'alloc A>,
    alloc: &'alloc A,
}

impl Default for SourceMap<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceMap<'_> {
    #[must_use]
    pub fn new() -> Self {
        Self { files: Vec::new_in(&Global), alloc: &Global }
    }
}

impl<'alloc, A: Allocator> SourceMap<'alloc, A> {
    #[must_use]
    pub fn new_in(alloc: &'alloc A) -> Self {
        Self { files: Vec::new_in(alloc), alloc }
    }

    /// Add a source file, returning its `FileId`.
    pub fn add_file(&mut self, name: Cow<'static, str>, source: Cow<'static, str>) -> FileId {
        let id = FileId::new(self.files.len());
        let line_starts = self.line_starts(&source);
        self.files.push(SourceFile { name, source, line_starts });
        id
    }

    /// Create a Span for a given file and byte range.
    /// Panics in debug if range is out of bounds.
    #[must_use]
    pub fn span(&self, file_id: FileId, range: Range<usize>) -> Span {
        debug_assert!(file_id.index() < self.files.len(), "file_id out of bounds");
        let source = &self.files[file_id.index()].source;
        debug_assert!(range.end <= source.len(), "range out of bounds");
        debug_assert!(
            source.is_char_boundary(range.start) && source.is_char_boundary(range.end),
            "range must be at character boundaries"
        );
        Span::new(file_id, range.into())
    }

    /// Access the source text for a file.
    #[must_use]
    pub fn source(&self, file_id: FileId) -> Option<&str> {
        self.files.get(file_id.index()).map(|f| &f.source[..])
    }

    /// Access the file name.
    #[must_use]
    pub fn filename(&self, file_id: FileId) -> Option<&str> {
        self.files.get(file_id.index()).map(|f| &f.name[..])
    }

    /// Convert a byte offset to line and column (1‑based).
    #[must_use]
    pub fn line_col(&self, file_id: FileId, offset: usize) -> Option<(usize, usize)> {
        let file = self.files.get(file_id.index())?;
        if offset > file.source.len() {
            return None;
        }
        let line_idx = match file.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        Some((line_idx + 1, offset - file.line_starts[line_idx] + 1))
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
        Some(&file.source[start..end])
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
    fn line_col_handles_multiline_offsets() {
        let mut source_map = SourceMap::new();
        let file = source_map.add_file("foo.rs".into(), "alpha\nbeta\ngamma".into());

        assert_eq!(source_map.line_col(file, 0), Some((1, 1)));
        assert_eq!(source_map.line_col(file, 5), Some((1, 6)));
        assert_eq!(source_map.line_col(file, 6), Some((2, 1)));
        assert_eq!(source_map.line_col(file, 10), Some((2, 5)));
        assert_eq!(source_map.line_col(file, 11), Some((3, 1)));
        assert_eq!(source_map.line_col(file, 16), Some((3, 6)));
        assert_eq!(source_map.line_col(file, 17), None);
    }

    #[test]
    fn line_and_line_start_use_one_based_lines() {
        let mut source_map = SourceMap::new();
        let file = source_map.add_file("foo.rs".into(), "alpha\nbeta\ngamma".into());

        assert_eq!(source_map.line(file, 1), Some("alpha"));
        assert_eq!(source_map.line(file, 2), Some("beta"));
        assert_eq!(source_map.line(file, 3), Some("gamma"));
        assert_eq!(source_map.line(file, 4), None);
        assert_eq!(source_map.line_start(file, 0), Some(0));
        assert_eq!(source_map.line_start(file, 6), Some(6));
        assert_eq!(source_map.line_start(file, 16), Some(11));
    }
}
