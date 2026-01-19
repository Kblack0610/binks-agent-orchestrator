//! System prompts for specialized agents
//!
//! Each agent has a carefully crafted system prompt that defines
//! its role, capabilities, and output format.

mod planner;
mod implementer;
mod reviewer;
mod investigator;
mod tester;

pub use planner::PLANNER_PROMPT;
pub use implementer::IMPLEMENTER_PROMPT;
pub use reviewer::REVIEWER_PROMPT;
pub use investigator::INVESTIGATOR_PROMPT;
pub use tester::TESTER_PROMPT;
