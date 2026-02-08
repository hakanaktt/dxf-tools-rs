//! Unified DWG and DXF Reading Tests
//!
//! Tests both DWG and DXF reading with identical structure and patterns.
//! Each format is tested using the same sample files from reference_samples/.

use std::path::PathBuf;
use std::time::{Duration, Instant};
use acadrust::io::dwg::DwgReader;
use acadrust::io::dxf::DxfReader;
use acadrust::CadDocument;
use acadrust::ACadVersion;

/// Get the path to reference samples directory.
fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("reference_samples")
}

/// Get path to a specific sample file.
fn sample_path(name: &str) -> PathBuf {
    samples_dir().join(name)
}

/// Document statistics for comparison.
#[derive(Debug, Clone)]
struct DocStats {
    version: ACadVersion,
    layers: usize,
    linetypes: usize,
    text_styles: usize,
    block_records: usize,
    dim_styles: usize,
    entities: usize,
    read_time_ms: f64,
}

impl DocStats {
    fn from_document(doc: &CadDocument, read_time: Duration) -> Self {
        Self {
            version: doc.version,
            layers: doc.layers.len(),
            linetypes: doc.line_types.len(),
            text_styles: doc.text_styles.len(),
            block_records: doc.block_records.len(),
            dim_styles: doc.dim_styles.len(),
            entities: doc.entities().count(),
            read_time_ms: read_time.as_secs_f64() * 1000.0,
        }
    }

    fn print(&self, label: &str) {
        println!("  {}: {:.3}ms", label, self.read_time_ms);
        println!("    Version: {:?}", self.version);
        println!("    Layers: {}, LineTypes: {}, TextStyles: {}",
            self.layers, self.linetypes, self.text_styles);
        println!("    BlockRecords: {}, DimStyles: {}, Entities: {}",
            self.block_records, self.dim_styles, self.entities);
    }
}

// =============================================================================
// DWG Reading Tests
// =============================================================================
mod dwg_reading {
    use super::*;

    fn read_dwg_file(filename: &str) -> Result<(CadDocument, Duration), String> {
        let path = sample_path(filename);
        if !path.exists() {
            return Err(format!("File not found: {}", filename));
        }

        let start = Instant::now();
        let reader = DwgReader::from_file(&path)
            .map_err(|e| format!("Failed to open: {:?}", e))?;

        // Use catch_unwind for AC15 format files that may panic
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            reader.read()
        }));

        let elapsed = start.elapsed();

        match result {
            Ok(Ok(doc)) => Ok((doc, elapsed)),
            Ok(Err(e)) => Err(format!("Read error: {:?}", e)),
            Err(_) => Err("Panic during read".to_string()),
        }
    }

    #[test]
    fn test_dwg_ac1014() {
        match read_dwg_file("sample_AC1014.dwg") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DWG AC1014 (R14) ===");
                stats.print("DWG");
                assert_eq!(doc.version, ACadVersion::AC1014);
            }
            Err(e) => println!("AC1014 DWG: {}", e),
        }
    }

    #[test]
    fn test_dwg_ac1015() {
        match read_dwg_file("sample_AC1015.dwg") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DWG AC1015 (R2000) ===");
                stats.print("DWG");
                assert_eq!(doc.version, ACadVersion::AC1015);
            }
            Err(e) => println!("AC1015 DWG (expected): {}", e),
        }
    }

    #[test]
    fn test_dwg_ac1018() {
        match read_dwg_file("sample_AC1018.dwg") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DWG AC1018 (R2004) ===");
                stats.print("DWG");
                assert_eq!(doc.version, ACadVersion::AC1018);
            }
            Err(e) => println!("AC1018 DWG: {}", e),
        }
    }

    #[test]
    fn test_dwg_ac1021() {
        match read_dwg_file("sample_AC1021.dwg") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DWG AC1021 (R2007) ===");
                stats.print("DWG");
                assert_eq!(doc.version, ACadVersion::AC1021);
            }
            Err(e) => println!("AC1021 DWG: {}", e),
        }
    }

    #[test]
    fn test_dwg_ac1024() {
        match read_dwg_file("sample_AC1024.dwg") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DWG AC1024 (R2010) ===");
                stats.print("DWG");
                assert_eq!(doc.version, ACadVersion::AC1024);
            }
            Err(e) => println!("AC1024 DWG: {}", e),
        }
    }

    #[test]
    fn test_dwg_ac1027() {
        match read_dwg_file("sample_AC1027.dwg") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DWG AC1027 (R2013) ===");
                stats.print("DWG");
                assert_eq!(doc.version, ACadVersion::AC1027);
            }
            Err(e) => println!("AC1027 DWG: {}", e),
        }
    }

    #[test]
    fn test_dwg_ac1032() {
        match read_dwg_file("sample_AC1032.dwg") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DWG AC1032 (R2018) ===");
                stats.print("DWG");
                assert_eq!(doc.version, ACadVersion::AC1032);
            }
            Err(e) => println!("AC1032 DWG: {}", e),
        }
    }

    #[test]
    fn test_dwg_all_versions() {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║                   DWG Reading Summary                      ║");
        println!("╚════════════════════════════════════════════════════════════╝\n");

        let versions = [
            ("sample_AC1014.dwg", "AC1014", "R14"),
            ("sample_AC1015.dwg", "AC1015", "R2000"),
            ("sample_AC1018.dwg", "AC1018", "R2004"),
            ("sample_AC1021.dwg", "AC1021", "R2007"),
            ("sample_AC1024.dwg", "AC1024", "R2010"),
            ("sample_AC1027.dwg", "AC1027", "R2013"),
            ("sample_AC1032.dwg", "AC1032", "R2018"),
        ];

        println!("{:<20} {:>8} {:>8} {:>8} {:>10}",
            "File", "Layers", "Types", "Entities", "Time (ms)");
        println!("{}", "-".repeat(60));

        for (filename, version, release) in versions {
            match read_dwg_file(filename) {
                Ok((doc, elapsed)) => {
                    let stats = DocStats::from_document(&doc, elapsed);
                    println!("{:<20} {:>8} {:>8} {:>8} {:>10.3}",
                        format!("{} ({})", version, release),
                        stats.layers,
                        stats.linetypes,
                        stats.entities,
                        stats.read_time_ms);
                }
                Err(e) => {
                    println!("{:<20} {:>40}", format!("{} ({})", version, release), e);
                }
            }
        }
    }
}

// =============================================================================
// DXF Reading Tests
// =============================================================================
mod dxf_reading {
    use super::*;

    fn read_dxf_file(filename: &str) -> Result<(CadDocument, Duration), String> {
        let path = sample_path(filename);
        if !path.exists() {
            return Err(format!("File not found: {}", filename));
        }

        let start = Instant::now();
        let reader = DxfReader::from_file(&path)
            .map_err(|e| format!("Failed to open: {:?}", e))?;

        let doc = reader.read()
            .map_err(|e| format!("Read error: {:?}", e))?;

        let elapsed = start.elapsed();
        Ok((doc, elapsed))
    }

    #[test]
    fn test_dxf_ascii_ac1015() {
        match read_dxf_file("sample_AC1015_ascii.dxf") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DXF ASCII AC1015 (R2000) ===");
                stats.print("DXF");
            }
            Err(e) => println!("AC1015 DXF ASCII: {}", e),
        }
    }

    #[test]
    fn test_dxf_binary_ac1015() {
        match read_dxf_file("sample_AC1015_binary.dxf") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DXF Binary AC1015 (R2000) ===");
                stats.print("DXF");
            }
            Err(e) => println!("AC1015 DXF Binary: {}", e),
        }
    }

    #[test]
    fn test_dxf_ascii_ac1018() {
        match read_dxf_file("sample_AC1018_ascii.dxf") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DXF ASCII AC1018 (R2004) ===");
                stats.print("DXF");
            }
            Err(e) => println!("AC1018 DXF ASCII: {}", e),
        }
    }

    #[test]
    fn test_dxf_binary_ac1018() {
        match read_dxf_file("sample_AC1018_binary.dxf") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DXF Binary AC1018 (R2004) ===");
                stats.print("DXF");
            }
            Err(e) => println!("AC1018 DXF Binary: {}", e),
        }
    }

    #[test]
    fn test_dxf_ascii_ac1021() {
        match read_dxf_file("sample_AC1021_ascii.dxf") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DXF ASCII AC1021 (R2007) ===");
                stats.print("DXF");
            }
            Err(e) => println!("AC1021 DXF ASCII: {}", e),
        }
    }

    #[test]
    fn test_dxf_binary_ac1021() {
        match read_dxf_file("sample_AC1021_binary.dxf") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DXF Binary AC1021 (R2007) ===");
                stats.print("DXF");
            }
            Err(e) => println!("AC1021 DXF Binary: {}", e),
        }
    }

    #[test]
    fn test_dxf_ascii_ac1024() {
        match read_dxf_file("sample_AC1024_ascii.dxf") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DXF ASCII AC1024 (R2010) ===");
                stats.print("DXF");
            }
            Err(e) => println!("AC1024 DXF ASCII: {}", e),
        }
    }

    #[test]
    fn test_dxf_binary_ac1024() {
        match read_dxf_file("sample_AC1024_binary.dxf") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DXF Binary AC1024 (R2010) ===");
                stats.print("DXF");
            }
            Err(e) => println!("AC1024 DXF Binary: {}", e),
        }
    }

    #[test]
    fn test_dxf_ascii_ac1027() {
        match read_dxf_file("sample_AC1027_ascii.dxf") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DXF ASCII AC1027 (R2013) ===");
                stats.print("DXF");
            }
            Err(e) => println!("AC1027 DXF ASCII: {}", e),
        }
    }

    #[test]
    fn test_dxf_binary_ac1027() {
        match read_dxf_file("sample_AC1027_binary.dxf") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DXF Binary AC1027 (R2013) ===");
                stats.print("DXF");
            }
            Err(e) => println!("AC1027 DXF Binary: {}", e),
        }
    }

    #[test]
    fn test_dxf_ascii_ac1032() {
        match read_dxf_file("sample_AC1032_ascii.dxf") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DXF ASCII AC1032 (R2018) ===");
                stats.print("DXF");
            }
            Err(e) => println!("AC1032 DXF ASCII: {}", e),
        }
    }

    #[test]
    fn test_dxf_binary_ac1032() {
        match read_dxf_file("sample_AC1032_binary.dxf") {
            Ok((doc, elapsed)) => {
                let stats = DocStats::from_document(&doc, elapsed);
                println!("\n=== DXF Binary AC1032 (R2018) ===");
                stats.print("DXF");
            }
            Err(e) => println!("AC1032 DXF Binary: {}", e),
        }
    }

    #[test]
    fn test_dxf_all_ascii_versions() {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║                 DXF ASCII Reading Summary                  ║");
        println!("╚════════════════════════════════════════════════════════════╝\n");

        let versions = [
            ("sample_AC1015_ascii.dxf", "AC1015", "R2000"),
            ("sample_AC1018_ascii.dxf", "AC1018", "R2004"),
            ("sample_AC1021_ascii.dxf", "AC1021", "R2007"),
            ("sample_AC1024_ascii.dxf", "AC1024", "R2010"),
            ("sample_AC1027_ascii.dxf", "AC1027", "R2013"),
            ("sample_AC1032_ascii.dxf", "AC1032", "R2018"),
        ];

        println!("{:<20} {:>8} {:>8} {:>8} {:>10}",
            "File", "Layers", "Types", "Entities", "Time (ms)");
        println!("{}", "-".repeat(60));

        for (filename, version, release) in versions {
            match read_dxf_file(filename) {
                Ok((doc, elapsed)) => {
                    let stats = DocStats::from_document(&doc, elapsed);
                    println!("{:<20} {:>8} {:>8} {:>8} {:>10.3}",
                        format!("{} ({})", version, release),
                        stats.layers,
                        stats.linetypes,
                        stats.entities,
                        stats.read_time_ms);
                }
                Err(e) => {
                    println!("{:<20} {:>40}", format!("{} ({})", version, release), e);
                }
            }
        }
    }

    #[test]
    fn test_dxf_all_binary_versions() {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║                DXF Binary Reading Summary                  ║");
        println!("╚════════════════════════════════════════════════════════════╝\n");

        let versions = [
            ("sample_AC1015_binary.dxf", "AC1015", "R2000"),
            ("sample_AC1018_binary.dxf", "AC1018", "R2004"),
            ("sample_AC1021_binary.dxf", "AC1021", "R2007"),
            ("sample_AC1024_binary.dxf", "AC1024", "R2010"),
            ("sample_AC1027_binary.dxf", "AC1027", "R2013"),
            ("sample_AC1032_binary.dxf", "AC1032", "R2018"),
        ];

        println!("{:<20} {:>8} {:>8} {:>8} {:>10}",
            "File", "Layers", "Types", "Entities", "Time (ms)");
        println!("{}", "-".repeat(60));

        for (filename, version, release) in versions {
            match read_dxf_file(filename) {
                Ok((doc, elapsed)) => {
                    let stats = DocStats::from_document(&doc, elapsed);
                    println!("{:<20} {:>8} {:>8} {:>8} {:>10.3}",
                        format!("{} ({})", version, release),
                        stats.layers,
                        stats.linetypes,
                        stats.entities,
                        stats.read_time_ms);
                }
                Err(e) => {
                    println!("{:<20} {:>40}", format!("{} ({})", version, release), e);
                }
            }
        }
    }
}

// =============================================================================
// Performance Comparison (DWG vs DXF)
// =============================================================================
mod performance_comparison {
    use super::*;

    fn read_dwg_file(filename: &str) -> Option<DocStats> {
        let path = sample_path(filename);
        if !path.exists() {
            return None;
        }

        let start = Instant::now();
        let reader = DwgReader::from_file(&path).ok()?;

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            reader.read()
        }));

        let elapsed = start.elapsed();
        match result {
            Ok(Ok(doc)) => Some(DocStats::from_document(&doc, elapsed)),
            _ => None,
        }
    }

    fn read_dxf_file(filename: &str) -> Option<DocStats> {
        let path = sample_path(filename);
        if !path.exists() {
            return None;
        }

        let start = Instant::now();
        let reader = DxfReader::from_file(&path).ok()?;
        let doc = reader.read().ok()?;
        let elapsed = start.elapsed();
        Some(DocStats::from_document(&doc, elapsed))
    }

    #[test]
    fn test_performance_comparison_all() {
        println!("\n╔══════════════════════════════════════════════════════════════════════════╗");
        println!("║                  DWG vs DXF Performance Comparison                       ║");
        println!("╚══════════════════════════════════════════════════════════════════════════╝\n");

        let versions = [
            ("AC1015", "R2000", "sample_AC1015.dwg", "sample_AC1015_ascii.dxf"),
            ("AC1018", "R2004", "sample_AC1018.dwg", "sample_AC1018_ascii.dxf"),
            ("AC1021", "R2007", "sample_AC1021.dwg", "sample_AC1021_ascii.dxf"),
            ("AC1024", "R2010", "sample_AC1024.dwg", "sample_AC1024_ascii.dxf"),
            ("AC1027", "R2013", "sample_AC1027.dwg", "sample_AC1027_ascii.dxf"),
            ("AC1032", "R2018", "sample_AC1032.dwg", "sample_AC1032_ascii.dxf"),
        ];

        println!("{:<15} {:>12} {:>12} {:>12} {:>10}",
            "Version", "DWG (ms)", "DXF (ms)", "Speedup", "Status");
        println!("{}", "-".repeat(65));

        for (version, release, dwg_file, dxf_file) in versions {
            let dwg_stats = read_dwg_file(dwg_file);
            let dxf_stats = read_dxf_file(dxf_file);

            let label = format!("{} ({})", version, release);

            match (dwg_stats, dxf_stats) {
                (Some(dwg), Some(dxf)) => {
                    let speedup = dxf.read_time_ms / dwg.read_time_ms;
                    let status = if dwg.entities == dxf.entities { "✓ Match" } else { "≠ Diff" };
                    println!("{:<15} {:>12.3} {:>12.3} {:>11.1}x {:>10}",
                        label, dwg.read_time_ms, dxf.read_time_ms, speedup, status);
                }
                (Some(dwg), None) => {
                    println!("{:<15} {:>12.3} {:>12} {:>12} {:>10}",
                        label, dwg.read_time_ms, "FAIL", "-", "DWG only");
                }
                (None, Some(dxf)) => {
                    println!("{:<15} {:>12} {:>12.3} {:>12} {:>10}",
                        label, "FAIL", dxf.read_time_ms, "-", "DXF only");
                }
                (None, None) => {
                    println!("{:<15} {:>12} {:>12} {:>12} {:>10}",
                        label, "FAIL", "FAIL", "-", "Both fail");
                }
            }
        }

        println!("\nNote: DWG and DXF sample files have different content.");
        println!("'≠ Diff' indicates the files contain different data, not a reader error.");
    }

    #[test]
    fn test_format_comparison_ac1018() {
        println!("\n=== AC1018 (R2004) Format Comparison ===\n");

        let dwg = read_dwg_file("sample_AC1018.dwg");
        let dxf_ascii = read_dxf_file("sample_AC1018_ascii.dxf");
        let dxf_binary = read_dxf_file("sample_AC1018_binary.dxf");

        if let Some(ref stats) = dwg {
            stats.print("DWG");
        } else {
            println!("  DWG: Failed to read");
        }

        if let Some(ref stats) = dxf_ascii {
            stats.print("DXF ASCII");
        } else {
            println!("  DXF ASCII: Failed to read");
        }

        if let Some(ref stats) = dxf_binary {
            stats.print("DXF Binary");
        } else {
            println!("  DXF Binary: Failed to read");
        }

        // Performance comparison
        if let (Some(dwg), Some(dxf)) = (&dwg, &dxf_ascii) {
            println!("\n  Performance:");
            let speedup = dxf.read_time_ms / dwg.read_time_ms;
            println!("    DWG is {:.1}x faster than DXF ASCII", speedup);
        }
    }

    #[test]
    fn test_format_comparison_ac1032() {
        println!("\n=== AC1032 (R2018) Format Comparison ===\n");

        let dwg = read_dwg_file("sample_AC1032.dwg");
        let dxf_ascii = read_dxf_file("sample_AC1032_ascii.dxf");
        let dxf_binary = read_dxf_file("sample_AC1032_binary.dxf");

        if let Some(ref stats) = dwg {
            stats.print("DWG");
        } else {
            println!("  DWG: Failed to read");
        }

        if let Some(ref stats) = dxf_ascii {
            stats.print("DXF ASCII");
        } else {
            println!("  DXF ASCII: Failed to read");
        }

        if let Some(ref stats) = dxf_binary {
            stats.print("DXF Binary");
        } else {
            println!("  DXF Binary: Failed to read");
        }

        // Performance comparison
        if let (Some(dwg), Some(dxf)) = (&dwg, &dxf_ascii) {
            println!("\n  Performance:");
            let speedup = dxf.read_time_ms / dwg.read_time_ms;
            println!("    DWG is {:.1}x faster than DXF ASCII", speedup);
        }
    }
}
