use std::collections::VecDeque;
use std::fmt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// 错误消息结构
#[derive(Debug, Serialize)]
pub struct ErrorMessage {
    #[serde(rename = "type")]
    msg_type: MessageType,
    payload: Option<String>,
}

// 消息类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageType {
    Open,
    Leave,
    Candidate,
    Offer,
    Answer,
    Expire,
    Heartbeat,
    #[serde(rename = "ID-TAKEN")]
    IdTaken,
    Error,
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageType::Open => write!(f, "OPEN"),
            MessageType::Leave => write!(f, "LEAVE"),
            MessageType::Candidate => write!(f, "CANDIDATE"),
            MessageType::Offer => write!(f, "OFFER"),
            MessageType::Answer => write!(f, "ANSWER"),
            MessageType::Expire => write!(f, "EXPIRE"),
            MessageType::Heartbeat => write!(f, "HEARTBEAT"),
            MessageType::IdTaken => write!(f, "ID-TAKEN"),
            MessageType::Error => write!(f, "ERROR"),
        }
    }
}

// 消息结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    pub src: Option<String>,
    pub dst: Option<String>,
    pub payload: Option<Value>,
}

impl Message {
    /// 创建新消息
    pub fn new(msg_type: MessageType, src: Option<String>, dst: Option<String>, payload: Option<Value>) -> Self {
        Self {
            msg_type,
            src,
            dst,
            payload,
        }
    }

    /// 创建带有payload的消息
    pub fn with_payload(msg_type: MessageType, src: Option<String>, dst: Option<String>, payload: Value) -> Self {
        Self::new(msg_type, src, dst, Some(payload))
    }

    /// 创建没有payload的消息
    pub fn without_payload(msg_type: MessageType, src: Option<String>, dst: Option<String>) -> Self {
        Self::new(msg_type, src, dst, None)
    }

    /// 检查是否是信令消息（OFFER, ANSWER, CANDIDATE）
    pub fn is_signaling(&self) -> bool {
        matches!(
            self.msg_type,
            MessageType::Offer | MessageType::Answer | MessageType::Candidate
        )
    }

    /// 检查是否是控制消息（OPEN, LEAVE, EXPIRE, HEARTBEAT, ID_TAKEN, ERROR）
    pub fn is_control(&self) -> bool {
        !self.is_signaling()
    }
}


