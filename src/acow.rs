use alloc::alloc::Allocator;
use alloc::boxed::Box;
use alloc::string::String;
use core::fmt;
use core::ops::Deref;

/// Allocator-aware cow string.
///
/// Like `Cow<'static, str>`, except owned string storage is allocated with `A`.
#[derive(Debug, PartialEq, Eq)]
pub enum Acow<'alloc, A: Allocator> {
    Borrowed(&'static str),
    Owned(Box<str, &'alloc A>),
}

impl<'alloc, A: Allocator> Acow<'alloc, A> {
    #[must_use]
    pub fn from_str_in(value: &str, alloc: &'alloc A) -> Self {
        Self::Owned(Box::clone_from_ref_in(value, alloc))
    }

    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Borrowed(value) => value,
            Self::Owned(value) => value,
        }
    }
}

impl<A: Allocator> Default for Acow<'_, A> {
    fn default() -> Self {
        Self::Borrowed("")
    }
}

impl<A: Allocator> From<&'static str> for Acow<'_, A> {
    fn from(value: &'static str) -> Self {
        Self::Borrowed(value)
    }
}

impl<'alloc, A: Allocator> From<Box<str, &'alloc A>> for Acow<'alloc, A> {
    fn from(value: Box<str, &'alloc A>) -> Self {
        Self::Owned(value)
    }
}

impl<A: Allocator> AsRef<str> for Acow<'_, A> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<A: Allocator> Deref for Acow<'_, A> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<A: Allocator> fmt::Display for Acow<'_, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub trait IntoAcow<'alloc, A: Allocator> {
    fn into_acow(self, alloc: &'alloc A) -> Acow<'alloc, A>;
}

impl<'alloc, A: Allocator> IntoAcow<'alloc, A> for &'static str {
    fn into_acow(self, _alloc: &'alloc A) -> Acow<'alloc, A> {
        Acow::Borrowed(self)
    }
}

impl<'alloc, A: Allocator> IntoAcow<'alloc, A> for String {
    fn into_acow(self, alloc: &'alloc A) -> Acow<'alloc, A> {
        Acow::from_str_in(&self, alloc)
    }
}

impl<'alloc, A: Allocator> IntoAcow<'alloc, A> for Box<str, &'alloc A> {
    fn into_acow(self, _alloc: &'alloc A) -> Acow<'alloc, A> {
        Acow::Owned(self)
    }
}

impl<'alloc, A: Allocator> IntoAcow<'alloc, A> for Acow<'alloc, A> {
    fn into_acow(self, _alloc: &'alloc A) -> Acow<'alloc, A> {
        self
    }
}
