use crate::preprocessor::RgbaPattern;

/// A unique identifier for a tile
pub type TileId = usize;

/// A list of type T indexable by TileId
pub type IdMap<T> = Vec<T>;

/// A tile is a wrapper around a pattern from the source image
/// with additional info
pub struct Tile {
    pub id: TileId,
    pub pattern: RgbaPattern,
    pub frequency: u32,
}
