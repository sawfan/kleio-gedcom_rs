use std::process::ExitCode;
use std::{env, fs};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("kleio-gedcom import failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), kleio_gedcom::DbError> {
    let mut args = env::args().skip(1);
    let Some(gedcom_path) = args.next() else {
        eprintln!(
            "Usage: cargo run -p kleio-gedcom --features db -- <path-to-file.ged> [project-name]\n\n\
Creates or opens kleio.sqlite, initializes the GEDCOM schema, creates a project, imports the GEDCOM, and prints the import id/hash.\n\n\
Library helpers are exposed by the `kleio_gedcom` crate."
        );
        return Ok(());
    };
    let project_name = args.next().unwrap_or_else(|| "Kleio Project".to_string());
    run_sqlite_example(&gedcom_path, &project_name)
}

fn run_sqlite_example(gedcom_path: &str, project_name: &str) -> Result<(), kleio_gedcom::DbError> {
    let mut conn = kleio_gedcom::open_database("kleio.sqlite")?;
    kleio_gedcom::init_schema(&conn)?;

    let project = kleio_gedcom::create_project(&conn, project_name)?;
    let gedcom_text = fs::read_to_string(gedcom_path)?;
    let filename = std::path::Path::new(gedcom_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("import.ged");
    let import = kleio_gedcom::import_gedcom_file(&mut conn, &project.id, filename, &gedcom_text)?;

    println!("project_id={}", project.id);
    println!("gedcom_import_id={}", import.id);
    println!("gedcom_file_hash={}", import.file_hash);

    Ok(())
}
