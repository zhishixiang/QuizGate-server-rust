use tokio::sync::mpsc;

pub struct Client{
    pub client_key:String,
    pub client_handler: mpsc::Sender<String>,
}
