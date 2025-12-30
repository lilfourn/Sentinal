//! XML Writer Module
//!
//! Generates token-optimized XML representations of compressed trees
//! for consumption by Claude and other AI models.

use super::{format_size, CompressedNode};

/// XML writer for compressed tree structures
pub struct XmlWriter {
    /// Whether to include full paths or just names
    pub include_full_paths: bool,
    /// Whether to include size information
    pub include_sizes: bool,
    /// Whether to include tags
    pub include_tags: bool,
    /// Indentation string (e.g., "  " for 2 spaces)
    pub indent: String,
}

impl Default for XmlWriter {
    fn default() -> Self {
        Self {
            include_full_paths: true,
            include_sizes: true,
            include_tags: true,
            indent: "  ".to_string(),
        }
    }
}

impl XmlWriter {
    /// Create a new XML writer with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate XML representation of a compressed tree
    ///
    /// Output format optimized for AI context:
    /// ```xml
    /// <folder path="/Downloads">
    ///   <file path="resume.pdf" vector_tags="career,document" size="1.2MB" />
    ///   <summary path="/Downloads/images" count="450" type="image"
    ///            common_tags="photo,screenshot" description="Mostly .jpg and .png from 2023-2024" />
    ///   <folder path="/Downloads/Projects">
    ///     ...
    ///   </folder>
    /// </folder>
    /// ```
    pub fn to_xml(&self, node: &CompressedNode) -> String {
        let mut output = String::new();
        self.write_node(node, 0, &mut output);
        output
    }

    /// Write a single node and its children
    fn write_node(&self, node: &CompressedNode, depth: usize, output: &mut String) {
        let indent = self.indent.repeat(depth);

        if node.is_collapsed {
            // Collapsed folder - write as summary element
            self.write_summary(node, &indent, output);
        } else if node.is_directory {
            // Expanded folder - write as folder element with children
            self.write_folder(node, depth, &indent, output);
        } else {
            // File - write as file element
            self.write_file(node, &indent, output);
        }
    }

    /// Write a file element
    fn write_file(&self, node: &CompressedNode, indent: &str, output: &mut String) {
        output.push_str(indent);
        output.push_str("<file");

        // Path attribute
        if self.include_full_paths {
            output.push_str(&format!(" path=\"{}\"", escape_xml(&node.path.to_string_lossy())));
        } else {
            output.push_str(&format!(" name=\"{}\"", escape_xml(&node.name)));
        }

        // Size attribute
        if self.include_sizes {
            if let Some(size) = node.size {
                output.push_str(&format!(" size=\"{}\"", format_size(size)));
            }
        }

        // Extension attribute
        if let Some(ref ext) = node.extension {
            output.push_str(&format!(" ext=\"{}\"", escape_xml(ext)));
        }

        // Tags attribute
        if self.include_tags && !node.tags.is_empty() {
            output.push_str(&format!(" vector_tags=\"{}\"", node.tags.join(",")));
        }

        output.push_str(" />\n");
    }

    /// Write an expanded folder element
    fn write_folder(&self, node: &CompressedNode, depth: usize, indent: &str, output: &mut String) {
        output.push_str(indent);
        output.push_str("<folder");

        // Path attribute
        if self.include_full_paths {
            output.push_str(&format!(" path=\"{}\"", escape_xml(&node.path.to_string_lossy())));
        } else {
            output.push_str(&format!(" name=\"{}\"", escape_xml(&node.name)));
        }

        // Tags attribute for folder
        if self.include_tags && !node.tags.is_empty() {
            output.push_str(&format!(" vector_tags=\"{}\"", node.tags.join(",")));
        }

        if node.children.is_empty() {
            output.push_str(" />\n");
        } else {
            output.push_str(">\n");

            // Write children
            for child in &node.children {
                self.write_node(child, depth + 1, output);
            }

            output.push_str(indent);
            output.push_str("</folder>\n");
        }
    }

    /// Write a collapsed folder as a summary element
    fn write_summary(&self, node: &CompressedNode, indent: &str, output: &mut String) {
        output.push_str(indent);
        output.push_str("<summary");

        // Path attribute
        if self.include_full_paths {
            output.push_str(&format!(" path=\"{}\"", escape_xml(&node.path.to_string_lossy())));
        } else {
            output.push_str(&format!(" name=\"{}\"", escape_xml(&node.name)));
        }

        if let Some(ref summary) = node.summary {
            // Count attribute
            output.push_str(&format!(" count=\"{}\"", summary.file_count));

            // Subdirs count if any
            if summary.dir_count > 0 {
                output.push_str(&format!(" subdirs=\"{}\"", summary.dir_count));
            }

            // Type attribute
            if let Some(ref primary_type) = summary.primary_type {
                output.push_str(&format!(" type=\"{}\"", escape_xml(primary_type)));
            }

            // Size attribute
            if self.include_sizes && summary.total_size > 0 {
                output.push_str(&format!(" total_size=\"{}\"", format_size(summary.total_size)));
            }

            // Common tags
            if self.include_tags && !summary.common_tags.is_empty() {
                output.push_str(&format!(" common_tags=\"{}\"", summary.common_tags.join(",")));
            }

            // Date range
            if let Some(ref date_range) = summary.date_range {
                output.push_str(&format!(" dates=\"{}\"", escape_xml(date_range)));
            }

            // Description
            output.push_str(&format!(" description=\"{}\"", escape_xml(&summary.description)));

            // Type breakdown as compact format
            if !summary.type_breakdown.is_empty() {
                let breakdown: Vec<String> = summary
                    .type_breakdown
                    .iter()
                    .take(5) // Limit to top 5 types
                    .map(|(ext, count)| format!("{}:{}", ext, count))
                    .collect();
                output.push_str(&format!(" breakdown=\"{}\"", breakdown.join(";")));
            }
        }

        output.push_str(" />\n");
    }

    /// Generate a compact XML representation (minimal attributes)
    ///
    /// Useful when token budget is very tight
    pub fn to_compact_xml(&self, node: &CompressedNode) -> String {
        let compact_writer = XmlWriter {
            include_full_paths: false,
            include_sizes: false,
            include_tags: false,
            indent: " ".to_string(),
        };
        compact_writer.to_xml(node)
    }
}

/// Escape special XML characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Convenience function to generate XML from a compressed node
pub fn to_xml(node: &CompressedNode) -> String {
    XmlWriter::new().to_xml(node)
}

/// Convenience function to generate compact XML from a compressed node
pub fn to_compact_xml(node: &CompressedNode) -> String {
    XmlWriter::new().to_compact_xml(node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::CollapsedSummary;
    use std::path::PathBuf;

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("hello & world"), "hello &amp; world");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_file_xml() {
        let node = CompressedNode::file(
            PathBuf::from("/test/file.pdf"),
            "file.pdf".to_string(),
            1_500_000,
            Some("pdf".to_string()),
            vec!["document".to_string()],
        );

        let xml = to_xml(&node);
        assert!(xml.contains("path=\"/test/file.pdf\""));
        assert!(xml.contains("size=\"1.4MB\""));
        assert!(xml.contains("ext=\"pdf\""));
        assert!(xml.contains("vector_tags=\"document\""));
    }

    #[test]
    fn test_folder_xml() {
        let child = CompressedNode::file(
            PathBuf::from("/test/folder/file.pdf"),
            "file.pdf".to_string(),
            1024,
            Some("pdf".to_string()),
            vec![],
        );

        let node = CompressedNode::folder(
            PathBuf::from("/test/folder"),
            "folder".to_string(),
            vec![child],
            vec![],
        );

        let xml = to_xml(&node);
        assert!(xml.contains("<folder path=\"/test/folder\">"));
        assert!(xml.contains("</folder>"));
        assert!(xml.contains("<file"));
    }

    #[test]
    fn test_summary_xml() {
        let summary = CollapsedSummary {
            file_count: 47,
            dir_count: 3,
            total_size: 1_500_000_000,
            primary_type: Some("image".to_string()),
            description: "47 image files, 23 PDF files".to_string(),
            common_tags: vec!["photo".to_string(), "screenshot".to_string()],
            date_range: Some("2023-01 to 2024-12".to_string()),
            type_breakdown: vec![("jpg".to_string(), 30), ("png".to_string(), 17)],
        };

        let node = CompressedNode::collapsed(
            PathBuf::from("/test/images"),
            "images".to_string(),
            summary,
            vec![],
        );

        let xml = to_xml(&node);
        assert!(xml.contains("<summary"));
        assert!(xml.contains("count=\"47\""));
        assert!(xml.contains("subdirs=\"3\""));
        assert!(xml.contains("type=\"image\""));
        assert!(xml.contains("total_size=\"1.4GB\""));
        assert!(xml.contains("common_tags=\"photo,screenshot\""));
        assert!(xml.contains("dates=\"2023-01 to 2024-12\""));
    }

    #[test]
    fn test_compact_xml() {
        let node = CompressedNode::file(
            PathBuf::from("/test/file.pdf"),
            "file.pdf".to_string(),
            1024,
            Some("pdf".to_string()),
            vec!["document".to_string()],
        );

        let compact = to_compact_xml(&node);
        assert!(compact.contains("name=\"file.pdf\""));
        assert!(!compact.contains("path=")); // Full path excluded
        assert!(!compact.contains("size=")); // Size excluded
        assert!(!compact.contains("vector_tags=")); // Tags excluded
    }
}
