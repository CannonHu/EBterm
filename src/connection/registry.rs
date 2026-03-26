use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::Connection;

pub type ConnectionRegistry = HashMap<String, Arc<Mutex<Box<dyn Connection>>>>;
