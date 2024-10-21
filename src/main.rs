#![feature(error_generic_member_access)]

pub mod codegen;
pub mod manifest;

use std::{
    backtrace::Backtrace,
    io::{Read, Write},
    path::PathBuf,
};

use clap::Parser;
use codegen::{AnimatedImage, Context, Image};
use colored::Colorize;
use manifest::Manifest;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("`{0}` was not found")]
    ManifestNotFound(PathBuf),
    #[error("image error `{0}`")]
    Image(#[from] image::ImageError),
    #[error("codegen error {0}")]
    CodeGen(#[from] tinytemplate::error::Error),
    #[error("failed to parse manifest {source}")]
    Manifest {
        #[from]
        source: toml::de::Error,
    },
    #[error("io error {source} {backtrace:?}")]
    Io {
        #[from]
        source: std::io::Error,
        backtrace: Backtrace,
    },
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Input manifest path
    #[arg(short, long, default_value = "manifest.toml")]
    input: PathBuf,

    /// Output path
    #[arg(short, long, default_value = "images.h")]
    output: PathBuf,
}

fn main() {
    let args = Args::parse();

    if let Err(e) = run(args) {
        println!("{}: {}", "error:".red(), e);
    }
}

fn run(args: Args) -> Result<(), Error> {
    // Ensure manifest exists
    if !std::fs::exists(&args.input)? {
        return Err(Error::ManifestNotFound(args.input));
    }

    // Read the manifest
    let mut f = std::fs::File::open(args.input)?;
    let mut input = String::default();
    f.read_to_string(&mut input)?;
    std::mem::drop(f);

    // Parse the manifest
    let Manifest {
        width,
        height,
        data_pin,
        images: manifest_images,
    } = toml::from_str(&input)?;

    let width = width.get();
    let height = height.get();

    // Process manifest
    let images: Vec<Image> = manifest_images
        .iter()
        .filter(|i| {
            !i.1.is_dir()
        })
        .filter(|i| {
            let extension = i.1.extension().unwrap().to_string_lossy();
            extension != "gif"
        })
        .map(|i| load_image(i.0.clone(), i.1.clone(), width, height))
        .collect::<Result<Vec<Image>, Error>>()?;

    let animated_images: Vec<AnimatedImage> = manifest_images
        .iter()
        .filter(|i| {
            if i.1.is_dir() {
                return true;
            }
            let extension = i.1.extension().unwrap().to_string_lossy();
            extension == "gif"
        })
        .map(|i| match i.1.is_dir() {
            true => load_dir(i.0.clone(), i.1.clone(), width, height),
            false => load_gif(i.0.clone(), i.1.clone(), width, height),
        })
        .collect::<Result<Vec<AnimatedImage>, Error>>()?;

    // Generate the code
    let context = Context {
        width,
        height,
        data_pin,
        images,
        animated_images,
    };

    let output = codegen::generate(context)?;

    // Write the code out to header file
    let mut f = std::fs::File::create(args.output)?;
    f.write_all(output.as_bytes())?;
    f.flush()?;

    Ok(())
}

fn load_image(name: String, path: PathBuf, width: u32, height: u32) -> Result<Image, Error> {
    let i = image::open(path)?;
    let i = i.resize(width, height, image::imageops::FilterType::CatmullRom);
    let rgba = i.into_rgba8();

    Ok(Image {
        name,
        pixels: rgba
            .pixels()
            .map(|pixel| u32::from_le_bytes(pixel.0) & 0xFFFFFF)
            .collect(),
    })
}

fn load_gif(name: String, path: PathBuf, width: u32, height: u32) -> Result<AnimatedImage, Error> {
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);

    let f = std::fs::File::open(path)?;
    let mut decoder = decoder.read_info(f).unwrap(); // TODO: handle unwrap
    let iw = decoder.width() as u32;
    let ih = decoder.height() as u32;

    let mut frames = vec![];

    let mut index = 0;
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        // TODO: handle unwrap
        let i = image::RgbaImage::from_vec(iw, ih, frame.buffer.to_vec()).unwrap(); // TODO: handle unwrap
        let i = image::imageops::resize(&i, width, height, image::imageops::FilterType::CatmullRom);

        frames.push(Image {
            name: name.clone() + &format!("_{}", index),
            pixels: i
                .pixels()
                .map(|pixel| u32::from_le_bytes(pixel.0) & 0xFFFFFF)
                .collect(),
        });

        index += 1;
    }

    Ok(AnimatedImage {
        name,
        frame_count: frames.len() as u32,
        frames,
    })
}

fn load_dir(name: String, path: PathBuf, width: u32, height: u32) -> Result<AnimatedImage, Error> {
    let dir = std::fs::read_dir(path)?;

    // TODO: handle errors better here
    let mut paths: Vec<PathBuf> = dir
        .filter_map(|file| file.ok())
        .filter(|file| file.file_type().map(|file| file.is_file()).unwrap_or(false))
        .map(|file| file.path())
        .collect();

    paths.sort();

    let frames: Vec<Image> = paths
        .into_iter()
        .enumerate()
        .map(|(i, path)| load_image(format!("{}_{}", name, i), path, width, height))
        .collect::<Result<Vec<Image>, Error>>()?;

    Ok(AnimatedImage {
        name,
        frame_count: frames.len() as u32,
        frames,
    })
}
