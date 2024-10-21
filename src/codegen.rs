use serde::Serialize;
use tinytemplate::TinyTemplate;

use crate::Error;

static HEADER_TEMPLATE: &'static str = include_str!("templates/header.tem");
static IMAGE_TEMPLATE: &'static str = include_str!("templates/image.tem");
static ANIMATED_IMAGE_TEMPLATE: &'static str = include_str!("templates/animated_image.tem");


#[derive(Serialize, Debug)]
pub struct Context {
    pub width: u32,
    pub height: u32,
    pub data_pin: u32,
    pub images: Vec<Image>,
    pub animated_images: Vec<AnimatedImage>,
}

#[derive(Serialize, Debug)]
pub struct Image {
    pub name: String,
    pub pixels: Vec<u32>,
}

#[derive(Serialize, Debug)]
pub struct AnimatedImage {
    pub name: String,
    pub frames: Vec<Image>,
    pub frame_count: u32,
}

pub fn generate(context: Context) -> Result<String, Error> {
    let mut tt = TinyTemplate::new();

    tt.add_template("image", IMAGE_TEMPLATE)?;
    tt.add_template("animated_image", ANIMATED_IMAGE_TEMPLATE)?;
    tt.add_template("header", HEADER_TEMPLATE)?;

    let output = tt.render("header", &context)?;

    Ok(output)
}