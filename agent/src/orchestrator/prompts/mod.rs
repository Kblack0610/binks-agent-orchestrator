//! System prompts for specialized agents
//!
//! Each agent has a carefully crafted system prompt that defines
//! its role, capabilities, and output format.

mod implementer;
mod investigator;
mod planner;
mod reviewer;
mod tester;

pub use implementer::IMPLEMENTER_PROMPT;
pub use investigator::INVESTIGATOR_PROMPT;
pub use planner::PLANNER_PROMPT;
pub use reviewer::REVIEWER_PROMPT;
pub use tester::TESTER_PROMPT;
