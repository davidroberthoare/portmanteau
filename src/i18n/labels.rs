//! Texts that come from data (a reason, a folder name) and are translated
//! when shown. Their literals are listed here so the template finds them.

use super::{tr, tr_dyn};

/// The reason the compile gives for leaving a document out, in the current language.
pub fn compile_reason(reason: &str) -> String {
    match reason {
        "Excluded from compile" => tr("Excluded from compile").to_string(),
        "In the Research folder" => tr("In the Research folder").to_string(),
        other => tr_dyn(other),
    }
}

/// A binder folder's name as shown: the program's own folders are translated.
pub fn folder_name(name: &str) -> String {
    match name {
        "Draft" => tr("Draft").to_string(),
        "Front Matter" => tr("Front Matter").to_string(),
        "Back Matter" => tr("Back Matter").to_string(),
        "Research" => tr("Research").to_string(),
        other => other.to_string(),
    }
}

/// The names of the interface text sizes.
pub fn scale_name(name: &str) -> String {
    match name {
        "Standard" => tr("Standard").to_string(),
        "Large" => tr("Large").to_string(),
        "Larger" => tr("Larger").to_string(),
        other => other.to_string(),
    }
}
