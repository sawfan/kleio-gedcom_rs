//! GEDCOM import/export integration for Kleio.

#[cfg(feature = "db")]
pub mod db;
pub mod import;
pub mod local_authoring;

#[cfg(feature = "db")]
pub use db::{
    CURRENT_SCHEMA_VERSION, DbError, GedcomImport, hash_gedcom_sha256, hash_gedcom_text,
    import_gedcom_file, init_schema, open_database,
};
#[cfg(feature = "db")]
pub use db::{
    GedcomImportSummary as DbGedcomImportSummary, PlaceResolution, Project, ProjectDocument,
    create_project, get_project_document, list_place_resolutions, save_project_document,
    upsert_place_resolution,
};

pub use import::*;
pub use local_authoring::*;
