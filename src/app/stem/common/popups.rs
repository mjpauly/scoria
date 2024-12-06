//! Messages sent to the frontend that trigger a popup, currently a toast.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PopUp {
    pub kind: PopUpKind,
    pub msg: String,
    pub code: PopUpCode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PopUpKind {
    Error,
    Warn,
    Info,
    Success,
}

/// A code used to associate this popup with a particular action, e.g. deleting
/// data points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PopUpCode {
    DeletePoints,
    CopyPoints,
    Other, // anything where the popup being shown doesn't trigger anything
}
