//! Collection of well-known handle references read from the DWG header.
//!
//! Ported from ACadSharp `DwgHeaderHandlesCollection.cs`.

use std::collections::HashMap;

use crate::document::HeaderVariables;

/// Stores handle references parsed from the DWG file header section.
///
/// Each field corresponds to a well-known handle that the header references
/// (layers, line-types, dimension styles, control objects, dictionaries, etc.).
/// Handles are stored in a `HashMap<String, u64>` keyed by the field name.
#[derive(Debug, Clone, Default)]
pub struct DwgHeaderHandlesCollection {
    handles: HashMap<String, u64>,
}

impl DwgHeaderHandlesCollection {
    /// Create an empty handle collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a handle by name.
    pub fn get(&self, name: &str) -> Option<u64> {
        self.handles.get(name).copied()
    }

    /// Set a handle by name.
    pub fn set(&mut self, name: &str, value: u64) {
        self.handles.insert(name.to_uppercase(), value);
    }

    /// Remove a handle by name.
    pub fn remove(&mut self, name: &str) -> Option<u64> {
        self.handles.remove(name)
    }

    /// Get all stored handles as a list.
    pub fn all_handles(&self) -> Vec<u64> {
        self.handles.values().copied().collect()
    }

    /// Number of stored handles.
    pub fn len(&self) -> usize {
        self.handles.len()
    }

    /// Whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }

    /// Update header variables from the resolved handles.
    ///
    /// This is the Rust equivalent of `DwgHeaderHandlesCollection.UpdateHeader`
    /// in ACadSharp. It resolves handle references into name strings on the
    /// header variables.
    ///
    /// # Arguments
    ///
    /// * `header`  - The header variables to update.
    /// * `resolve` - A closure that resolves a handle to a name string, or
    ///               `None` if the handle is not found.
    pub fn update_header<F>(&self, header: &mut HeaderVariables, mut resolve: F)
    where
        F: FnMut(u64) -> Option<String>,
    {
        if let Some(handle) = self.get("CLAYER") {
            if let Some(name) = resolve(handle) {
                header.current_layer_name = name;
            }
        }

        if let Some(handle) = self.get("CELTYPE") {
            if let Some(name) = resolve(handle) {
                header.current_linetype_name = name;
            }
        }

        if let Some(handle) = self.get("CMLSTYLE") {
            if let Some(name) = resolve(handle) {
                header.multiline_style = name;
            }
        }

        if let Some(handle) = self.get("TEXTSTYLE") {
            if let Some(name) = resolve(handle) {
                header.current_text_style_name = name;
            }
        }

        if let Some(handle) = self.get("DIMSTYLE") {
            if let Some(name) = resolve(handle) {
                header.current_dimstyle_name = name;
            }
        }

        // Dimension text style, block names, etc. can be added as the
        // header variable struct gains those fields.
    }
}

/// Well-known handle field names used in the DWG header.
///
/// These match the C# property names in `DwgHeaderHandlesCollection`.
pub mod handle_names {
    pub const CMATERIAL: &str = "CMATERIAL";
    pub const CLAYER: &str = "CLAYER";
    pub const TEXTSTYLE: &str = "TEXTSTYLE";
    pub const CELTYPE: &str = "CELTYPE";
    pub const DIMSTYLE: &str = "DIMSTYLE";
    pub const CMLSTYLE: &str = "CMLSTYLE";
    pub const UCSNAME_PSPACE: &str = "UCSNAME_PSPACE";
    pub const UCSNAME_MSPACE: &str = "UCSNAME_MSPACE";
    pub const PUCSORTHOREF: &str = "PUCSORTHOREF";
    pub const PUCSBASE: &str = "PUCSBASE";
    pub const UCSORTHOREF: &str = "UCSORTHOREF";
    pub const DIMTXSTY: &str = "DIMTXSTY";
    pub const DIMLDRBLK: &str = "DIMLDRBLK";
    pub const DIMBLK: &str = "DIMBLK";
    pub const DIMBLK1: &str = "DIMBLK1";
    pub const DIMBLK2: &str = "DIMBLK2";
    pub const DICTIONARY_LAYOUTS: &str = "DICTIONARY_LAYOUTS";
    pub const DICTIONARY_PLOTSETTINGS: &str = "DICTIONARY_PLOTSETTINGS";
    pub const DICTIONARY_PLOTSTYLES: &str = "DICTIONARY_PLOTSTYLES";
    pub const CPSNID: &str = "CPSNID";
    pub const PAPER_SPACE: &str = "PAPER_SPACE";
    pub const MODEL_SPACE: &str = "MODEL_SPACE";
    pub const BYLAYER: &str = "BYLAYER";
    pub const BYBLOCK: &str = "BYBLOCK";
    pub const CONTINUOUS: &str = "CONTINUOUS";
    pub const DIMLTYPE: &str = "DIMLTYPE";
    pub const DIMLTEX1: &str = "DIMLTEX1";
    pub const DIMLTEX2: &str = "DIMLTEX2";
    pub const VIEWPORT_ENTITY_HEADER_CONTROL_OBJECT: &str =
        "VIEWPORT_ENTITY_HEADER_CONTROL_OBJECT";
    pub const DICTIONARY_ACAD_GROUP: &str = "DICTIONARY_ACAD_GROUP";
    pub const DICTIONARY_ACAD_MLINESTYLE: &str = "DICTIONARY_ACAD_MLINESTYLE";
    pub const DICTIONARY_NAMED_OBJECTS: &str = "DICTIONARY_NAMED_OBJECTS";
    pub const BLOCK_CONTROL_OBJECT: &str = "BLOCK_CONTROL_OBJECT";
    pub const LAYER_CONTROL_OBJECT: &str = "LAYER_CONTROL_OBJECT";
    pub const STYLE_CONTROL_OBJECT: &str = "STYLE_CONTROL_OBJECT";
    pub const LINETYPE_CONTROL_OBJECT: &str = "LINETYPE_CONTROL_OBJECT";
    pub const VIEW_CONTROL_OBJECT: &str = "VIEW_CONTROL_OBJECT";
    pub const UCS_CONTROL_OBJECT: &str = "UCS_CONTROL_OBJECT";
    pub const VPORT_CONTROL_OBJECT: &str = "VPORT_CONTROL_OBJECT";
    pub const APPID_CONTROL_OBJECT: &str = "APPID_CONTROL_OBJECT";
    pub const DIMSTYLE_CONTROL_OBJECT: &str = "DIMSTYLE_CONTROL_OBJECT";
    pub const DICTIONARY_MATERIALS: &str = "DICTIONARY_MATERIALS";
    pub const DICTIONARY_COLORS: &str = "DICTIONARY_COLORS";
    pub const DICTIONARY_VISUALSTYLE: &str = "DICTIONARY_VISUALSTYLE";
    pub const INTERFEREOBJVS: &str = "INTERFEREOBJVS";
    pub const INTERFEREVPVS: &str = "INTERFEREVPVS";
    pub const DRAGVS: &str = "DRAGVS";
    pub const UCSBASE: &str = "UCSBASE";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let mut handles = DwgHeaderHandlesCollection::new();
        handles.set("CLAYER", 0x42);
        assert_eq!(handles.get("CLAYER"), Some(0x42));
    }

    #[test]
    fn test_missing_handle() {
        let handles = DwgHeaderHandlesCollection::new();
        assert_eq!(handles.get("NONEXISTENT"), None);
    }

    #[test]
    fn test_all_handles() {
        let mut handles = DwgHeaderHandlesCollection::new();
        handles.set("A", 1);
        handles.set("B", 2);
        let all = handles.all_handles();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&1));
        assert!(all.contains(&2));
    }

    #[test]
    fn test_update_header() {
        let mut handles = DwgHeaderHandlesCollection::new();
        handles.set("CLAYER", 10);
        handles.set("CELTYPE", 20);

        let mut header = HeaderVariables::default();

        handles.update_header(&mut header, |h| match h {
            10 => Some("MyLayer".to_string()),
            20 => Some("DASHED".to_string()),
            _ => None,
        });

        assert_eq!(header.current_layer_name, "MyLayer");
        assert_eq!(header.current_linetype_name, "DASHED");
    }
}
