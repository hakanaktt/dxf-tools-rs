//! DWG writer configuration.

/// Configuration options for the DWG writer.
#[derive(Debug, Clone)]
pub struct DwgWriterConfiguration {
    /// Whether to write XRecord objects.
    pub write_xrecords: bool,
    /// Whether to write extended data (XDATA).
    pub write_xdata: bool,
    /// Whether to write Shape objects.
    pub write_shapes: bool,
    /// Whether to close the output stream when done.
    pub close_stream: bool,
}

impl Default for DwgWriterConfiguration {
    fn default() -> Self {
        Self {
            write_xrecords: true,
            write_xdata: true,
            write_shapes: true,
            close_stream: true,
        }
    }
}
