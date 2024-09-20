use std::fs::File;
use actix_web::HttpResponse;
use serde_json::json;

pub fn read_file(file_path: &String) -> actix_web::Result<File, std::io::Error> {
    let mut file = match File::open(&file_path) {
        Ok(file) => file,
        Err(e) => {
            return Err(e);
        },
    };
    Ok(file)
}