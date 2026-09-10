//! Port of `Jikan\Request\Schedule\ScheduleRequest`.

use crate::request::{MalRequest, BASE_URL};

/// `Jikan\Request\Schedule\ScheduleRequest`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScheduleRequest;

impl ScheduleRequest {
    pub fn new() -> Self {
        ScheduleRequest
    }
}

impl MalRequest for ScheduleRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime/season/schedule")
    }
}
