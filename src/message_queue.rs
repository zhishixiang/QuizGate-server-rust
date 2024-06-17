use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Serialize, Deserialize)]
struct MessageQueue {
    messages: Vec<String>,
}

pub fn write_message_to_json_file(filename: &str, message: &str) -> io::Result<()> {
    let mut queue = read_messages_from_json_file(filename).unwrap_or(MessageQueue { messages: vec![] });

    queue.messages.push(message.to_string());

    let serialized = serde_json::to_string(&queue)?;
    let mut file = OpenOptions::new().create(true).write(true).truncate(true).open(filename)?;
    file.write_all(serialized.as_bytes())?;
    Ok(())
}

pub fn read_messages_from_json_file(filename: &str) -> io::Result<MessageQueue> {
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let queue: MessageQueue = serde_json::from_str(&contents)?;
    Ok(queue)
}
