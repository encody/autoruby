//! Annotation selection.

use crate::dictionary::TextEntry;

/// Annotation selector.
pub trait Select {
    /// Selects an annotation from the given list of candidates.
    fn select<'a>(&self, annotations: &[&'a TextEntry]) -> Option<&'a TextEntry>;
}

impl<T> Select for T
where
    T: for<'a> Fn(&[&'a TextEntry]) -> Option<&'a TextEntry>,
{
    fn select<'a>(&self, annotations: &[&'a TextEntry]) -> Option<&'a TextEntry> {
        self(annotations)
    }
}

pub mod heuristic {
    //! Annotation selection heuristics.

    use crate::dictionary::TextEntry;

    /// Selects the top annotation every time.
    #[must_use]
    pub fn all<'a>(annotations: &[&'a TextEntry]) -> Option<&'a TextEntry> {
        annotations.get(0).copied()
    }

    /// Only selects the top annotation if it is uncommon.
    #[must_use]
    pub fn uncommon_only<'a>(annotations: &[&'a TextEntry]) -> Option<&'a TextEntry> {
        match annotations.get(0) {
            Some(entry) if !entry.text_is_common && !entry.reading_is_common => Some(entry),
            _ => None,
        }
    }
}
