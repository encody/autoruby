use crate::dictionary::TextEntry;

pub trait AnnotationSelector {
    fn select<'a>(&self, annotations: &[&'a TextEntry]) -> Option<&'a TextEntry>;
}

pub struct FirstAnnotationSelector;

impl AnnotationSelector for FirstAnnotationSelector {
    fn select<'a>(&self, annotations: &[&'a TextEntry]) -> Option<&'a TextEntry> {
        annotations.get(0).copied()
    }
}

pub struct UncommonOnlyFirstAnnotationSelector;

impl AnnotationSelector for UncommonOnlyFirstAnnotationSelector {
    fn select<'a>(&self, annotations: &[&'a TextEntry]) -> Option<&'a TextEntry> {
        match annotations.get(0) {
            Some(entry) if !entry.text_is_common && !entry.reading_is_common => Some(entry),
            _ => None,
        }
    }
}
