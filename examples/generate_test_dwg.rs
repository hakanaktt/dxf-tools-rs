//! Generate a DWG file with various entities for testing in real CAD applications.
//!
//! Run with: cargo run --example generate_test_dwg
//!
//! Opens in: AutoCAD, BricsCAD, LibreCAD, ODA File Converter, etc.

use acadrust::entities::*;
use acadrust::types::{Color, Handle, Vector2, Vector3};
use acadrust::{CadDocument, DwgWriter};

fn main() {
    let mut doc = CadDocument::new();
    let mut handle = 0x100u64;

    // ── Helper closure for sequential handles ──
    let mut next_handle = || {
        let h = Handle::new(handle);
        handle += 1;
        h
    };

    // ═══════════════════════════════════════════════════════════════
    // 1. LINES — a simple grid
    // ═══════════════════════════════════════════════════════════════
    for i in 0..=5 {
        let y = i as f64 * 20.0;
        let mut line = Line::new();
        line.common.handle = next_handle();
        line.common.color = Color::ByLayer;
        line.start = Vector3::new(0.0, y, 0.0);
        line.end = Vector3::new(100.0, y, 0.0);
        doc.add_entity(EntityType::Line(line));
    }
    for i in 0..=5 {
        let x = i as f64 * 20.0;
        let mut line = Line::new();
        line.common.handle = next_handle();
        line.start = Vector3::new(x, 0.0, 0.0);
        line.end = Vector3::new(x, 100.0, 0.0);
        doc.add_entity(EntityType::Line(line));
    }

    // ═══════════════════════════════════════════════════════════════
    // 2. CIRCLES
    // ═══════════════════════════════════════════════════════════════
    let radii = [5.0, 10.0, 15.0];
    for (i, &r) in radii.iter().enumerate() {
        let mut circle = Circle::new();
        circle.common.handle = next_handle();
        circle.center = Vector3::new(150.0, 50.0, 0.0);
        circle.radius = r;
        circle.common.color = Color::Index(i as u8 + 1); // red, yellow, green
        doc.add_entity(EntityType::Circle(circle));
    }

    // ═══════════════════════════════════════════════════════════════
    // 3. ARCS
    // ═══════════════════════════════════════════════════════════════
    let mut arc = Arc::new();
    arc.common.handle = next_handle();
    arc.center = Vector3::new(200.0, 50.0, 0.0);
    arc.radius = 20.0;
    arc.start_angle = 0.0;
    arc.end_angle = std::f64::consts::PI * 1.5; // 270°
    arc.common.color = Color::Index(4);
    doc.add_entity(EntityType::Arc(arc));

    // ═══════════════════════════════════════════════════════════════
    // 4. ELLIPSE
    // ═══════════════════════════════════════════════════════════════
    let mut ellipse = Ellipse::new();
    ellipse.common.handle = next_handle();
    ellipse.center = Vector3::new(260.0, 50.0, 0.0);
    ellipse.major_axis = Vector3::new(25.0, 0.0, 0.0);
    ellipse.minor_axis_ratio = 0.4;
    ellipse.start_parameter = 0.0;
    ellipse.end_parameter = std::f64::consts::TAU;
    ellipse.common.color = Color::Index(5);
    doc.add_entity(EntityType::Ellipse(ellipse));

    // ═══════════════════════════════════════════════════════════════
    // 5. POINTS
    // ═══════════════════════════════════════════════════════════════
    let point_positions = [
        (10.0, 120.0),
        (30.0, 120.0),
        (50.0, 120.0),
        (70.0, 120.0),
        (90.0, 120.0),
    ];
    for &(x, y) in &point_positions {
        let mut pt = Point::new();
        pt.common.handle = next_handle();
        pt.location = Vector3::new(x, y, 0.0);
        doc.add_entity(EntityType::Point(pt));
    }

    // ═══════════════════════════════════════════════════════════════
    // 6. TEXT
    // ═══════════════════════════════════════════════════════════════
    let mut text = Text::new();
    text.common.handle = next_handle();
    text.value = "AcadRust DWG Writer Test".to_string();
    text.insertion_point = Vector3::new(0.0, 130.0, 0.0);
    text.height = 5.0;
    text.common.color = Color::Index(7);
    doc.add_entity(EntityType::Text(text));

    let mut text2 = Text::new();
    text2.common.handle = next_handle();
    text2.value = "Hello from Rust!".to_string();
    text2.insertion_point = Vector3::new(0.0, 140.0, 0.0);
    text2.height = 3.5;
    text2.rotation = 0.0;
    text2.common.color = Color::Index(1);
    doc.add_entity(EntityType::Text(text2));

    // ═══════════════════════════════════════════════════════════════
    // 7. MTEXT (multiline text)
    // ═══════════════════════════════════════════════════════════════
    let mut mtext = MText::with_value(
        "Multi-line text\\PSecond line\\PThird line",
        Vector3::new(0.0, 160.0, 0.0),
    )
    .with_height(3.0)
    .with_width(60.0);
    mtext.common.handle = next_handle();
    mtext.common.color = Color::Index(3);
    doc.add_entity(EntityType::MText(mtext));

    // ═══════════════════════════════════════════════════════════════
    // 8. LWPOLYLINE — rectangle
    // ═══════════════════════════════════════════════════════════════
    let mut rect = LwPolyline::from_points(vec![
        Vector2::new(120.0, 0.0),
        Vector2::new(140.0, 0.0),
        Vector2::new(140.0, 30.0),
        Vector2::new(120.0, 30.0),
    ]);
    rect.is_closed = true;
    rect.common.handle = next_handle();
    rect.common.color = Color::Index(6);
    doc.add_entity(EntityType::LwPolyline(rect));

    // ═══════════════════════════════════════════════════════════════
    // 9. LWPOLYLINE — rounded shape with bulges
    // ═══════════════════════════════════════════════════════════════
    let mut rounded = LwPolyline::new();
    rounded.common.handle = next_handle();
    rounded.common.color = Color::Index(2);
    rounded.is_closed = true;
    rounded.add_point(Vector2::new(120.0, 40.0));
    rounded.add_point_with_bulge(Vector2::new(140.0, 40.0), 0.5);
    rounded.add_point(Vector2::new(140.0, 60.0));
    rounded.add_point_with_bulge(Vector2::new(120.0, 60.0), 0.5);
    doc.add_entity(EntityType::LwPolyline(rounded));

    // ═══════════════════════════════════════════════════════════════
    // 10. SOLID (filled triangle)
    // ═══════════════════════════════════════════════════════════════
    let mut solid = Solid::new(
        Vector3::new(120.0, 70.0, 0.0),
        Vector3::new(140.0, 70.0, 0.0),
        Vector3::new(130.0, 90.0, 0.0),
        Vector3::new(130.0, 90.0, 0.0), // triangular solid
    );
    solid.common.handle = next_handle();
    solid.common.color = Color::Index(4);
    doc.add_entity(EntityType::Solid(solid));

    // ═══════════════════════════════════════════════════════════════
    // 11. 3D FACE
    // ═══════════════════════════════════════════════════════════════
    let mut face = Face3D::new(
        Vector3::new(50.0, -30.0, 0.0),
        Vector3::new(80.0, -30.0, 0.0),
        Vector3::new(80.0, -10.0, 10.0),
        Vector3::new(50.0, -10.0, 10.0),
    );
    face.common.handle = next_handle();
    face.common.color = Color::Index(3);
    doc.add_entity(EntityType::Face3D(face));

    // ═══════════════════════════════════════════════════════════════
    // 12. SPLINE — smooth curve
    // ═══════════════════════════════════════════════════════════════
    let mut spline = Spline::from_control_points(
        3,
        vec![
            Vector3::new(0.0, -10.0, 0.0),
            Vector3::new(20.0, -30.0, 0.0),
            Vector3::new(40.0, -5.0, 0.0),
            Vector3::new(60.0, -25.0, 0.0),
            Vector3::new(80.0, -10.0, 0.0),
        ],
    );
    spline.common.handle = next_handle();
    spline.common.color = Color::Index(1); // red
    // Generate uniform knot vector for degree 3 with 5 control points = 9 knots
    spline.knots = vec![0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0, 1.0];
    doc.add_entity(EntityType::Spline(spline));

    // ═══════════════════════════════════════════════════════════════
    // 13. LEADER
    // ═══════════════════════════════════════════════════════════════
    let mut leader = Leader::from_vertices(vec![
        Vector3::new(150.0, 90.0, 0.0),  // arrow tip
        Vector3::new(170.0, 100.0, 0.0), // bend
        Vector3::new(195.0, 100.0, 0.0), // end (by text)
    ]);
    leader.common.handle = next_handle();
    leader.common.color = Color::Index(7);
    leader.hookline_enabled = true;
    doc.add_entity(EntityType::Leader(leader));

    // ═══════════════════════════════════════════════════════════════
    // 14. DIMENSION — linear
    // ═══════════════════════════════════════════════════════════════
    let mut dim_linear = DimensionLinear::new(
        Vector3::new(0.0, -40.0, 0.0),
        Vector3::new(50.0, -40.0, 0.0),
    );
    dim_linear.base.common.handle = next_handle();
    dim_linear.base.definition_point = Vector3::new(50.0, -50.0, 0.0);
    dim_linear.base.text_middle_point = Vector3::new(25.0, -50.0, 0.0);
    dim_linear.set_offset(10.0);
    doc.add_entity(EntityType::Dimension(Dimension::Linear(dim_linear)));

    // ═══════════════════════════════════════════════════════════════
    // 15. DIMENSION — aligned
    // ═══════════════════════════════════════════════════════════════
    let mut dim_aligned = DimensionAligned::new(
        Vector3::new(0.0, -60.0, 0.0),
        Vector3::new(40.0, -75.0, 0.0),
    );
    dim_aligned.base.common.handle = next_handle();
    dim_aligned.base.definition_point = Vector3::new(43.0, -70.0, 0.0);
    dim_aligned.base.text_middle_point = Vector3::new(20.0, -72.0, 0.0);
    doc.add_entity(EntityType::Dimension(Dimension::Aligned(dim_aligned)));

    // ═══════════════════════════════════════════════════════════════
    // 16. DIMENSION — radius
    // ═══════════════════════════════════════════════════════════════
    let mut dim_radius = DimensionRadius::new(
        Vector3::new(200.0, 50.0, 0.0), // center of the arc we drew earlier
        Vector3::new(220.0, 50.0, 0.0), // point on arc
    );
    dim_radius.base.common.handle = next_handle();
    dim_radius.base.text_middle_point = Vector3::new(210.0, 55.0, 0.0);
    doc.add_entity(EntityType::Dimension(Dimension::Radius(dim_radius)));

    // ═══════════════════════════════════════════════════════════════
    // 17. RAY & XLINE (construction geometry)
    // ═══════════════════════════════════════════════════════════════
    let mut ray = Ray::new(
        Vector3::new(-10.0, 50.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
    );
    ray.common.handle = next_handle();
    ray.common.color = Color::Index(8);
    doc.add_entity(EntityType::Ray(ray));

    let mut xline = XLine::new(
        Vector3::new(50.0, -100.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    xline.common.handle = next_handle();
    xline.common.color = Color::Index(8);
    doc.add_entity(EntityType::XLine(xline));

    // ═══════════════════════════════════════════════════════════════
    // 18. TOLERANCE
    // ═══════════════════════════════════════════════════════════════
    let mut tol = Tolerance::new();
    tol.common.handle = next_handle();
    tol.insertion_point = Vector3::new(200.0, 110.0, 0.0);
    tol.direction = Vector3::new(1.0, 0.0, 0.0);
    tol.text = "{\\Fgdt;j}%%v{\\Fgdt;n}0.05%%v%%v%%v".to_string();
    doc.add_entity(EntityType::Tolerance(tol));

    // ═══════════════════════════════════════════════════════════════
    // WRITE FILE
    // ═══════════════════════════════════════════════════════════════
    let entity_count = doc.entity_count();
    println!("Document contains {} entities", entity_count);

    let writer = DwgWriter::new();

    // Write to current directory
    let output_path = std::path::Path::new("acadrust_test_output.dwg");
    match writer.write_to_file(&doc, output_path) {
        Ok(()) => {
            let size = std::fs::metadata(output_path).unwrap().len();
            println!(
                "Successfully wrote: {} ({} bytes / {:.1} KB)",
                output_path.display(),
                size,
                size as f64 / 1024.0
            );
            println!("\nOpen this file in AutoCAD, BricsCAD, or any DWG viewer to verify.");

            // Also print absolute path for convenience
            if let Ok(abs) = std::fs::canonicalize(output_path) {
                println!("Full path: {}", abs.display());
            }
        }
        Err(e) => {
            eprintln!("ERROR writing DWG: {}", e);
            std::process::exit(1);
        }
    }
}
