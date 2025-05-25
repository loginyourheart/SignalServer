use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::room::message::Message;

// 消息队列 trait
pub trait Queue: Send + Sync {
    fn add_message(&mut self, message: Message);
    fn get_messages(&self) -> Vec<Message>;
    fn clear(&mut self);
    fn len(&self) -> usize;

    fn read_message(&mut self) -> Option<Message>;

    fn get_last_read_time(&self) -> u64;
}

// 消息队列实现
#[derive(Debug)]
pub struct MessageQueue {
    messages: VecDeque<Message>,
    last_read_time: u64,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self {
            messages: VecDeque::new(),
            last_read_time: SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() as u64,
        }
    }
}

impl Queue for MessageQueue {
    fn add_message(&mut self, message: Message) {
        self.messages.push_back(message);
    }

    fn get_messages(&self) -> Vec<Message> {
        self.messages.iter().cloned().collect()
    }

    fn clear(&mut self) {
        self.messages.clear();
    }

    fn len(&self) -> usize {
        self.messages.len()
    }

    fn read_message(&mut self) -> Option<Message> {
        if let Some(message) = self.messages.pop_front() {
            // 获取当前时间戳
            self.last_read_time =  SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() as u64;
            Some(message)
        } else {
            None
        }
    }

    fn get_last_read_time(&self) -> u64 {
        self.last_read_time
    }
}
