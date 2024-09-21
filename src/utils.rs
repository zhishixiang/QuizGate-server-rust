use std::fs::File;
use std::path::Path;

pub fn read_file(file_path: &String) -> actix_web::Result<File, std::io::Error> {
    let file = match File::open(&file_path) {
        Ok(file) => file,
        Err(e) => {
            return Err(e);
        },
    };
    Ok(file)
}

// 检测tests目录下是否有对应问卷
pub fn is_test_id_exist(test_id: u32) -> bool {
    let file_path = format!("tests/{}.json", test_id);
    Path::new(&file_path).exists()
}