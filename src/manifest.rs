use std::{collections::HashMap, num::NonZeroU32, path::PathBuf};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Manifest {
    pub width: NonZeroU32,
    pub height: NonZeroU32,
    pub data_pin: u32,
    pub images: HashMap<String, PathBuf>,
}