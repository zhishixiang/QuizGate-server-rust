use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]

pub struct Respond{
    pub(crate) code:i8,
    pub(crate) msg:String
}
