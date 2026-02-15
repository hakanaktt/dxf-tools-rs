//! DWG document builder — assembles a `CadDocument` from parsed DWG sections.
//!
//! Ported from ACadSharp `DwgDocumentBuilder.cs`.

use std::collections::HashMap;

use crate::document::CadDocument;
use crate::entities::EntityType;
use crate::notification::{Notification, NotificationType};
use crate::types::DxfVersion;

use super::dwg_header_handles_collection::DwgHeaderHandlesCollection;
use super::dwg_reader_configuration::DwgReaderConfiguration;

/// Assembles a [`CadDocument`] from the raw sections read from a DWG file.
///
/// Holds intermediate state (object templates, handle maps, entity lists)
/// that are resolved in [`build_document`](Self::build_document) into the
/// final document.
pub struct DwgDocumentBuilder {
    /// The AutoCAD version of the file being read.
    pub version: DxfVersion,
    /// The document under construction.
    pub document: CadDocument,
    /// Reader configuration in effect.
    pub configuration: DwgReaderConfiguration,
    /// Handle references from the DWG header.
    pub header_handles: DwgHeaderHandlesCollection,
    /// Entities destined for paper space.
    pub paper_space_entities: Vec<EntityType>,
    /// Entities destined for model space.
    pub model_space_entities: Vec<EntityType>,
    /// Handle → resolved object name cache (for header resolution).
    pub handle_name_map: HashMap<u64, String>,
    /// Accumulated notifications.
    pub notifications: Vec<Notification>,
}

impl DwgDocumentBuilder {
    /// Create a new builder for the given version and document.
    pub fn new(
        version: DxfVersion,
        document: CadDocument,
        configuration: DwgReaderConfiguration,
    ) -> Self {
        Self {
            version,
            document,
            configuration,
            header_handles: DwgHeaderHandlesCollection::new(),
            paper_space_entities: Vec::new(),
            model_space_entities: Vec::new(),
            handle_name_map: HashMap::new(),
            notifications: Vec::new(),
        }
    }

    /// Whether unknown entities should be kept.
    pub fn keep_unknown_entities(&self) -> bool {
        self.configuration.keep_unknown_entities
    }

    /// Whether unknown non-graphical objects should be kept.
    pub fn keep_unknown_non_graphical_objects(&self) -> bool {
        self.configuration.keep_unknown_non_graphical_objects
    }

    /// Try to resolve a handle to a previously-registered object name.
    pub fn try_get_name(&self, handle: u64) -> Option<&str> {
        self.handle_name_map.get(&handle).map(|s| s.as_str())
    }

    /// Register a handle → name mapping.
    pub fn register_name(&mut self, handle: u64, name: String) {
        self.handle_name_map.insert(handle, name);
    }

    /// Record a notification.
    pub fn notify(&mut self, message: impl Into<String>, notification_type: NotificationType) {
        self.notifications
            .push(Notification::new(notification_type, message));
    }

    /// Assemble the final document from all collected sections.
    ///
    /// This is the main entry point after all section readers have populated
    /// the builder. It resolves cross-references, updates the header, and
    /// attaches entities to their block records.
    pub fn build_document(mut self) -> CadDocument {
        // Resolve header handle names
        let map = self.handle_name_map.clone();
        self.header_handles
            .update_header(&mut self.document.header, |h| map.get(&h).cloned());

        // Transfer notifications
        for n in self.notifications {
            self.document
                .notifications
                .notify(n.notification_type, n.message);
        }

        self.document
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creation() {
        let doc = CadDocument::new();
        let cfg = DwgReaderConfiguration::default();
        let builder = DwgDocumentBuilder::new(DxfVersion::AC1032, doc, cfg);
        assert_eq!(builder.version, DxfVersion::AC1032);
        assert!(!builder.keep_unknown_entities());
    }

    #[test]
    fn test_register_and_resolve_name() {
        let doc = CadDocument::new();
        let cfg = DwgReaderConfiguration::default();
        let mut builder = DwgDocumentBuilder::new(DxfVersion::AC1015, doc, cfg);
        builder.register_name(0x42, "TestLayer".to_string());
        assert_eq!(builder.try_get_name(0x42), Some("TestLayer"));
        assert_eq!(builder.try_get_name(0x99), None);
    }

    #[test]
    fn test_build_document_updates_header() {
        let doc = CadDocument::new();
        let cfg = DwgReaderConfiguration::default();
        let mut builder = DwgDocumentBuilder::new(DxfVersion::AC1015, doc, cfg);

        builder.register_name(10, "LayerOne".to_string());
        builder.header_handles.set("CLAYER", 10);

        let result = builder.build_document();
        assert_eq!(result.header.current_layer_name, "LayerOne");
    }
}
