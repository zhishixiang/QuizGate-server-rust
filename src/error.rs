use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub struct DuplicateConnectionsError;

#[derive(Debug)]
pub struct NoSuchValueError;

#[derive(Debug)]
pub struct CreateSqlPoolError;

impl Display for NoSuchValueError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "客户端提供了一个无效key")
    }
}

impl Display for DuplicateConnectionsError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "存在使用当前key的在线客户端") // user-facing output
    }
}

impl Display for CreateSqlPoolError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "创建sql池失败") // user-facing output
    }
}

impl Error for NoSuchValueError {}
impl Error for DuplicateConnectionsError {}

unsafe impl Send for DuplicateConnectionsError {}
unsafe impl Sync for DuplicateConnectionsError {}

unsafe impl Send for NoSuchValueError {}
unsafe impl Sync for NoSuchValueError {}

unsafe impl Send for CreateSqlPoolError {}
unsafe impl Sync for CreateSqlPoolError {}
