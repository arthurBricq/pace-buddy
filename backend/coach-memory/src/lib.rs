mod classifier;
mod coach;
mod context;
mod normalizer;
mod store;
mod updater;

pub use classifier::{MemoryClassifier, MemoryClassifierOutput};
pub use coach::{CoachMemory, CoachToolExecutor, COACH_SYSTEM_PROMPT, COACH_TOOL_PROMPT};
pub use context::{build_coach_context, CoachContextBundle};
pub use normalizer::{MemoryNormalizer, MemoryNormalizerOutput};
pub use store::CoachMemoryDataStore;
pub use updater::update_memory_after_exchange;
