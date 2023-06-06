use crate::dictionary::TextEntry;

pub trait Select {
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
    use crate::dictionary::TextEntry;

    pub fn all<'a>(annotations: &[&'a TextEntry]) -> Option<&'a TextEntry> {
        annotations.get(0).copied()
    }

    pub fn uncommon_only<'a>(annotations: &[&'a TextEntry]) -> Option<&'a TextEntry> {
        match annotations.get(0) {
            Some(entry) if !entry.text_is_common && !entry.reading_is_common => Some(entry),
            _ => None,
        }
    }
}
