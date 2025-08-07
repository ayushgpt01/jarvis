use super::{Message, MessageRole};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Context {
    messages: VecDeque<Message>,
    max_history: usize,
}

#[allow(dead_code)]
impl Context {
    pub fn new(max_history: usize) -> Self {
        Self {
            messages: VecDeque::new(),
            max_history,
        }
    }

    pub fn add_messages(&mut self, messages: Vec<Message>) {
        for message in messages {
            self.add_message(message.role, message.content);
        }
    }

    fn trim(&mut self) {
        while self.messages.len() > self.max_history {
            self.messages.pop_front();
        }
    }

    pub fn add_message(&mut self, role: MessageRole, content: String) -> &Message {
        let message = Message {
            role,
            content,
            metadata: None,
        };
        self.messages.push_back(message);

        // Add memory management support here

        self.trim();

        // Usually should never panic since we enforce max history to 0
        self.messages.back().unwrap()
    }

    pub fn add_user_message(&mut self, content: String) -> &Message {
        self.add_message(MessageRole::User, content)
    }

    pub fn add_assistant_message(&mut self, content: String) -> &Message {
        self.add_message(MessageRole::Assistant, content)
    }

    pub fn add_system_message(&mut self, content: String) -> &Message {
        self.add_message(MessageRole::System, content)
    }

    pub fn get_messages(&self) -> Vec<Message> {
        self.messages.iter().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn get_last_user_prompt(&self) -> Option<String> {
        self.messages
            .iter()
            .rev()
            .find(|msg| matches!(msg.role, MessageRole::User))
            .map(|msg| msg.content.clone())
    }
}
