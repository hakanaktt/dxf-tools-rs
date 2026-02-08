//! DWG Writer Integration Tests

use acadrust::{CadDocument, DwgWriter};
use acadrust::types::{Handle, Vector3};
use acadrust::entities::*;

#[test]
fn test_dwg_writer_produces_valid_bytes() {
    let doc = create_test_document();
    let writer = DwgWriter::new();
    let result = writer.write(&doc);
    assert!(result.is_ok(), "DwgWriter::write failed: {:?}", result.err());

    let bytes = result.unwrap();
    assert_eq!(&bytes[0..6], b"AC1018", "Version string mismatch");
    assert!(bytes.len() > 0x100, "File too small: {} bytes", bytes.len());
    println!("DWG file size: {} bytes", bytes.len());
}

#[test]
fn test_dwg_writer_to_file() {
    let doc = create_test_document();
    let writer = DwgWriter::new();

    let path = std::env::temp_dir().join("acadrust_test_output.dwg");
    let result = writer.write_to_file(&doc, &path);
    assert!(result.is_ok(), "write_to_file failed: {:?}", result.err());

    assert!(path.exists(), "Output file not created");
    let metadata = std::fs::metadata(&path).unwrap();
    assert!(metadata.len() > 256, "File too small: {} bytes", metadata.len());
    println!("Wrote DWG file: {} ({} bytes)", path.display(), metadata.len());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_dwg_writer_empty_document() {
    let doc = CadDocument::new();
    let writer = DwgWriter::new();
    let result = writer.write(&doc);
    assert!(result.is_ok(), "Empty doc write failed: {:?}", result.err());

    let bytes = result.unwrap();
    assert_eq!(&bytes[0..6], b"AC1018");
    assert!(bytes.len() > 0x100);
    println!("Empty DWG: {} bytes", bytes.len());
}

#[test]
fn test_dwg_writer_roundtrip_basic() {
    let mut doc = CadDocument::new();

    let mut line = Line::new();
    line.common.handle = Handle::new(0x100);
    line.start = Vector3 { x: 0.0, y: 0.0, z: 0.0 };
    line.end = Vector3 { x: 100.0, y: 50.0, z: 0.0 };
    doc.add_entity(EntityType::Line(line));

    let mut circle = Circle::new();
    circle.common.handle = Handle::new(0x101);
    circle.center = Vector3 { x: 50.0, y: 50.0, z: 0.0 };
    circle.radius = 25.0;
    doc.add_entity(EntityType::Circle(circle));

    let writer = DwgWriter::new();
    let bytes = writer.write(&doc).expect("Write failed");
    println!("Roundtrip DWG: {} bytes, {} entities written", bytes.len(), doc.entity_count());

    assert_eq!(&bytes[0..6], b"AC1018");
    assert_eq!(bytes[0x0C], 0x03);

    // Encrypted section should not be all zeros
    let encrypted_nonzero = bytes[0x80..0x100].iter().any(|&b| b != 0);
    assert!(encrypted_nonzero, "Encrypted header section all zeros");

    println!("File structure valid");
}

#[test]
fn test_dwg_writer_various_entities() {
    let mut doc = CadDocument::new();
    let mut hc = 0x200u64;

    let mut arc = Arc::new();
    arc.common.handle = Handle::new(hc); hc += 1;
    arc.center = Vector3 { x: 10.0, y: 10.0, z: 0.0 };
    arc.radius = 5.0;
    arc.start_angle = 0.0;
    arc.end_angle = std::f64::consts::PI;
    doc.add_entity(EntityType::Arc(arc));

    let mut point = Point::new();
    point.common.handle = Handle::new(hc); hc += 1;
    point.location = Vector3 { x: 25.0, y: 30.0, z: 0.0 };
    doc.add_entity(EntityType::Point(point));

    let mut ellipse = Ellipse::new();
    ellipse.common.handle = Handle::new(hc); hc += 1;
    ellipse.center = Vector3 { x: 50.0, y: 50.0, z: 0.0 };
    ellipse.major_axis = Vector3 { x: 20.0, y: 0.0, z: 0.0 };
    ellipse.minor_axis_ratio = 0.5;
    doc.add_entity(EntityType::Ellipse(ellipse));

    let mut text = Text::new();
    text.common.handle = Handle::new(hc); hc += 1;
    text.value = "Hello DWG".to_string();
    text.insertion_point = Vector3 { x: 0.0, y: 0.0, z: 0.0 };
    text.height = 2.5;
    doc.add_entity(EntityType::Text(text));

    let solid = Solid::new(
        Vector3 { x: 0.0, y: 0.0, z: 0.0 },
        Vector3 { x: 10.0, y: 0.0, z: 0.0 },
        Vector3 { x: 10.0, y: 10.0, z: 0.0 },
        Vector3 { x: 0.0, y: 10.0, z: 0.0 },
    );
    // Solid doesn't have common.handle directly visible, set via EntityType
    let mut solid_etype = EntityType::Solid(solid);
    solid_etype.as_entity_mut().set_handle(Handle::new(hc)); hc += 1;
    doc.add_entity(solid_etype);

    let mut ray = Ray::new(
        Vector3 { x: 0.0, y: 0.0, z: 0.0 },
        Vector3 { x: 1.0, y: 0.0, z: 0.0 },
    );
    ray.common.handle = Handle::new(hc); hc += 1;
    doc.add_entity(EntityType::Ray(ray));

    let mut xline = XLine::new(
        Vector3 { x: 0.0, y: 0.0, z: 0.0 },
        Vector3 { x: 0.0, y: 1.0, z: 0.0 },
    );
    xline.common.handle = Handle::new(hc);
    doc.add_entity(EntityType::XLine(xline));

    let writer = DwgWriter::new();
    let result = writer.write(&doc);
    assert!(result.is_ok(), "Various entities write failed: {:?}", result.err());

    let bytes = result.unwrap();
    println!("Various entities DWG: {} bytes, {} entities", bytes.len(), doc.entity_count());
    assert_eq!(&bytes[0..6], b"AC1018");
}

fn create_test_document() -> CadDocument {
    let mut doc = CadDocument::new();
    let mut line = Line::new();
    line.common.handle = Handle::new(0x50);
    line.start = Vector3 { x: 0.0, y: 0.0, z: 0.0 };
    line.end = Vector3 { x: 10.0, y: 10.0, z: 0.0 };
    doc.add_entity(EntityType::Line(line));
    doc
}
