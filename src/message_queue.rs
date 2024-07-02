use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::ops::Deref;
use std::thread::sleep;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json;

use crate::ClientList;
use crate::structs::request::Request;

#[derive(Serialize, Deserialize)]
struct MessageQueue {
    messages: Vec<Request>,
}

pub async fn create_mq_thread(client_list: ClientList){
    todo!();
    let filename = "queue.json";
    loop{
        let mut queue:MessageQueue = read_messages_from_json_file(filename).unwrap_or(MessageQueue { messages: vec![] });
        let client_list = client_list.lock().await.deref();
        sleep(Duration::from_secs(4));
    }
}
pub fn write_message_to_json_file(message: Request) -> io::Result<()> {
    let filename = "queue.json";
    let mut queue:MessageQueue = read_messages_from_json_file(filename).unwrap_or(MessageQueue { messages: vec![] });

    queue.messages.push(message);

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
