use super::*;

pub(crate) fn bundle_manuscript(project: &Project) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = ZipWriter::new(cursor);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let json = serde_json::to_string_pretty(project).map_err(|e| e.to_string())?;
        zip.start_file("project.json", opts).unwrap();
        zip.write_all(json.as_bytes()).unwrap();
        zip.start_file("manuscript.md", opts).unwrap();
        zip.write_all(markdown_manuscript(project, &project.export).as_bytes())
            .unwrap();
        zip.start_file("screenplay.fountain", opts).unwrap();
        zip.write_all(fountain_manuscript(project, &project.export).as_bytes())
            .unwrap();
        let mut notes = String::new();
        for doc in &project.documents {
            if !doc.snapshots.is_empty() {
                notes.push_str(&format!(
                    "# {}\n\n{} snapshots\n\n",
                    doc.title,
                    doc.snapshots.len()
                ));
                for snap in &doc.snapshots {
                    notes.push_str(&format!("- {} · {} words · {}\n", snap.name, snap.words, snap.created));
                }
            }
        }
        zip.start_file("version-notes.md", opts).unwrap();
        zip.write_all(notes.as_bytes()).unwrap();
        zip.start_file("README.txt", opts).unwrap();
        zip.write_all(
            format!(
                "{} by {}\n\nThis bundle contains the project file, a Markdown manuscript, a Fountain screenplay, and version notes.\nOpen project.json with Portmanteau to continue writing.\n",
                project.title, project.author
            )
            .as_bytes(),
        )
        .unwrap();
        zip.finish().unwrap();
    }
    Ok(buf)
}
