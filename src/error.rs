use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub struct DuplicateConnectionsError;

#[derive(Debug)]
pub struct NoSuchKeyError;

#[derive(Debug)]
pub struct ConnectionClosedError;

impl Display for NoSuchKeyError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "客户端提供了一个无效key")
    }
}

impl Display for DuplicateConnectionsError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "存在使用当前key的在线客户端") // user-facing output
    }
}
impl Error for NoSuchKeyError {}
impl Error for DuplicateConnectionsError {}

unsafe impl Send for DuplicateConnectionsError {}
unsafe impl Sync for DuplicateConnectionsError {}

unsafe impl Send for NoSuchKeyError {}
unsafe impl Sync for NoSuchKeyError {}

unsafe impl Send for ConnectionClosedError {}
unsafe impl Sync for ConnectionClosedError {}