pub mod preprocessor;
pub mod adjacency_rules;
pub mod wfc;

/// A unique identifier for a tile
pub type TileId = usize;

/// A list of type T indexable by TileId
pub type IdMap<T> = Vec<T>;
