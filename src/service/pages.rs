use std::path::PathBuf;
use actix_files::NamedFile;

pub(crate) async fn index() -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open(PathBuf::from("templates/exam.html"))?)
}

pub(crate) async fn upload_page() -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open(PathBuf::from("templates/upload.html"))?)
}

pub(crate) async fn register_page() -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open(PathBuf::from("templates/register.html"))?)
}