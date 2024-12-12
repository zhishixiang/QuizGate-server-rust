use std::path::PathBuf;
use actix_files::NamedFile;
use actix_web::HttpRequest;


// 静态资源
pub(crate) async fn resources(req: HttpRequest) -> actix_web::Result<NamedFile> {
    let mut path: PathBuf = PathBuf::from("resources/");
    let filename: String = req.match_info().query("filename").parse()?;
    path.push(filename);
    Ok(NamedFile::open(path)?)
}