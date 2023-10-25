//! Annotation selection.

use crate::{annotate::AnnotatedTextFragment, dictionary::TextEntry};

/// Annotation selector.
pub trait Select<'a> {
    /// Selects an annotation from the given list of candidates.
    fn select(&'_ self, fragment: &'a AnnotatedTextFragment<'a>) -> Option<&'a TextEntry>;
}

pub mod heuristic {
    //! Annotation selection heuristics.

    use crate::{annotate::AnnotatedTextFragment, dictionary::TextEntry};

    use super::Select;

    /// Selects the top annotation every time.
    pub struct All;

    impl<'a> Select<'a> for All {
        fn select(&self, fragment: &AnnotatedTextFragment<'a>) -> Option<&'a TextEntry> {
            fragment.annotations.get(0).copied()
        }
    }

    /// Only selects the top annotation if it is uncommon.
    pub struct UncommonOnly;

    impl<'a> Select<'a> for UncommonOnly {
        fn select(&self, fragment: &AnnotatedTextFragment<'a>) -> Option<&'a TextEntry> {
            match fragment.annotations.get(0) {
                Some(entry) if !entry.text_is_common && !entry.reading_is_common => Some(entry),
                _ => None,
            }
        }
    }
}

pub mod filter {
    //! Annotation filters.

    use std::{
        collections::HashSet,
        sync::{Arc, RwLock},
    };

    use crate::{annotate::AnnotatedTextFragment, dictionary::TextEntry};

    use super::Select;

    /// Filters out annotations that have already been seen.
    #[derive(Clone, Debug)]
    pub struct FirstOccurrence<'a, S: Select<'a>> {
        seen: Arc<RwLock<HashSet<&'a str>>>,
        selector: S,
    }

    impl<'a, S: Select<'a>> FirstOccurrence<'a, S> {
        /// Creates a new annotation selector that only selects the first occurrence of each
        /// annotation.
        pub fn new(selector: S) -> Self {
            Self {
                seen: Arc::default(),
                selector,
            }
        }
    }

    impl<'a, S: Select<'a>> Select<'a> for FirstOccurrence<'a, S> {
        fn select(&'_ self, fragment: &'a AnnotatedTextFragment<'a>) -> Option<&'a TextEntry> {
            let mut set = self.seen.write().unwrap();
            if (*set).insert(&fragment.text) {
                self.selector.select(fragment)
            } else {
                None
            }
        }
    }
}
