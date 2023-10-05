use ic_stable_structures::{DefaultMemoryManager, DefaultMemoryResourceType, DefaultMemoryType};

pub type MemoryType = DefaultMemoryType;

thread_local! {
    pub static MEMORY_MANAGER: DefaultMemoryManager = DefaultMemoryManager::init(DefaultMemoryResourceType::default());
}
