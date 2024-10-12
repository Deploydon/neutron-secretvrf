use cw_storage_plus::{Item, Map};


pub const RANDOM_OUTCOMES: Map<&str, u64> = Map::new("random_outcomes");
pub const JOBCOUNT: Item<u64> = Item::new("jobcount");