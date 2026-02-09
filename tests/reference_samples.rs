use acadrust::io::dxf::{DxfReader, DxfReaderConfiguration};
use std::path::Path;

fn read_sample(path: &str) -> String {
    let file = Path::new(path);
    let name = file.file_name().unwrap().to_str().unwrap();

    let config = DxfReaderConfiguration { failsafe: true };
    let reader = match DxfReader::from_file(path) {
        Ok(r) => r.with_configuration(config),
        Err(e) => return format!("{name}: OPEN ERROR: {e}"),
    };

    match reader.read() {
        Ok(doc) => {
            let version = format!("{:?}", doc.version);
            let entities = doc.entities().count();
            let layers = doc.layers.iter().count();
            let blocks = doc.block_records.iter().count();
            let objects = doc.objects.len();
            let notifs = doc.notifications.len();

            let mut line = format!(
                "{name}: OK  version={version}  entities={entities}  layers={layers}  blocks={blocks}  objects={objects}"
            );
            if notifs > 0 {
                line.push_str(&format!("  notifications={notifs}"));
                for n in doc.notifications.iter().take(5) {
                    line.push_str(&format!("\n    [{:?}] {}", n.notification_type, n.message));
                }
                if notifs > 5 {
                    line.push_str(&format!("\n    ... and {} more", notifs - 5));
                }
            }
            line
        }
        Err(e) => format!("{name}: READ ERROR: {e}"),
    }
}

#[test]
fn test_reference_samples() {
    let sample_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("reference_samples");
    let mut results = Vec::new();
    let mut failures = Vec::new();

    let mut entries: Vec<_> = std::fs::read_dir(&sample_dir)
        .expect("reference_samples directory not found")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "dxf")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let path = entry.path();
        let result = read_sample(path.to_str().unwrap());
        let is_error = result.contains("ERROR");
        if is_error {
            failures.push(result.clone());
        }
        results.push(result);
    }

    println!("\n=== Reference Sample Results ({} files) ===\n", results.len());
    for r in &results {
        println!("{r}");
    }

    if !failures.is_empty() {
        println!("\n=== FAILURES ({}) ===\n", failures.len());
        for f in &failures {
            println!("{f}");
        }
        panic!("{} files failed to read", failures.len());
    }
}
