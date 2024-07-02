use std::error::Error;
use std::fmt;
use std::fmt::Debug;

#[derive(Debug)]

pub struct NoSuchKeyError;
#[derive(Debug)]

pub struct ConnectionClosedError;
impl fmt::Display for NoSuchKeyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "客户端提供了一个无效key") // user-facing output
    }
}
impl fmt::Display for ConnectionClosedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "链接异常关闭") // user-facing output
    }
}

impl Error for ConnectionClosedError {}
impl Error for NoSuchKeyError {}
