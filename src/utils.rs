use std::fs::File;

pub fn read_file(file_path: &String) -> actix_web::Result<File, std::io::Error> {
    let file = match File::open(&file_path) {
        Ok(file) => file,
        Err(e) => {
            return Err(e);
        },
    };
    Ok(file)
}