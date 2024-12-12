use serde_json::Value;
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


// 干得好，我要给你打易佰昏！
pub fn mark(answer: &Vec<Value>, paper_info: &Value) -> i64 {
    let mut score: i64 = 0;
    let questions = paper_info["questions"].as_array().unwrap();
    for (i, question) in questions.iter().enumerate() {
        // 多选题
        if question["type"] == "multiple" {
            // 回答完全正确
            if answer[i] == question["correct"] {
                score += question["score"][1].as_i64().unwrap();
                // 部分内容正确且回答不为空
            } else if answer[i].as_i64() < question["correct"].as_i64() && !answer[i].is_null() {
                score += question["score"][0].as_i64().unwrap();
            }
            // 单选题
        } else if question["type"] == "radio" {
            if answer[i] == question["correct"] {
                score += question["score"].as_i64().unwrap();
            }
        }
    }
    score
}