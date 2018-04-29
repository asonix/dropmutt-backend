use std::{fs::File, io::BufReader, path::{Path, PathBuf}};

use actix::prelude::*;
use image;
use mime;
use mime_guess;

use error::DropmuttError;
use models;

pub struct ImageProcessor;

impl Actor for ImageProcessor {
    type Context = SyncContext<Self>;
}

fn get_format(path: &Path) -> Result<image::ImageFormat, DropmuttError> {
    let mt = mime_guess::guess_mime_type(path);

    if mt == mime::IMAGE_BMP {
        Ok(image::ImageFormat::BMP)
    } else if mt == mime::IMAGE_GIF {
        Ok(image::ImageFormat::GIF)
    } else if mt == mime::IMAGE_JPEG {
        Ok(image::ImageFormat::JPEG)
    } else if mt == mime::IMAGE_PNG {
        Ok(image::ImageFormat::PNG)
    } else {
        Err(DropmuttError::ImageProcessing)
    }
}

fn get_filename<P: AsRef<Path>>(path: P) -> Result<String, DropmuttError> {
    let path = path.as_ref();
    let base_filename = path.file_name()
        .and_then(|os_str| os_str.to_str().to_owned())
        .ok_or(DropmuttError::ImageProcessing)?;

    let extension = path.extension()
        .and_then(|os_str| os_str.to_str().to_owned())
        .ok_or(DropmuttError::ImageProcessing)?;

    let split_index = base_filename.len() - extension.len();

    if split_index < 2 {
        return Err(DropmuttError::ImageProcessing);
    }

    let (filename, _) = base_filename.split_at(split_index - 1);

    Ok(filename.to_owned())
}

fn image_path(filename: &str, width: &str, directory: &Path, full: bool) -> PathBuf {
    if full {
        directory.join(&format!("{}-{}.png", filename, width))
    } else {
        directory.join(&format!("{}-{}-thumb.png", filename, width))
    }
}

fn resize_image(
    img: &image::DynamicImage,
    width: u32,
    filename: &str,
    directory: &Path,
) -> Result<(PathBuf, i32, i32), DropmuttError> {
    let img = img.thumbnail(width, 10_000_000);

    let path = image_path(filename, &format!("{}", width), directory, false);

    let rgba_img = img.to_rgba();

    info!("Saving image: {:?}", path);
    img.save(&path)?;

    Ok((path, rgba_img.width() as i32, rgba_img.height() as i32))
}

impl Handler<ProcessImage> for ImageProcessor {
    type Result = Result<ProcessResponse, DropmuttError>;

    fn handle(&mut self, msg: ProcessImage, _: &mut Self::Context) -> Self::Result {
        let file_model = msg.0;

        let filename = get_filename(file_model.as_ref())?;

        let mut directory: PathBuf = file_model.as_ref().to_owned();
        directory.pop();

        let file = File::open(file_model.as_ref())?;
        let reader = BufReader::new(file);
        let img = image::load(reader, get_format(file_model.as_ref())?)?;

        let rgbaimg = img.to_rgba();

        let width = rgbaimg.width() as i32;
        let height = rgbaimg.height() as i32;

        info!("Processing image: {}x{}", width, height);

        let mut files = Vec::new();

        if width > 200 {
            files.push(resize_image(&img, 200, &filename, &directory)?);
        }

        if width > 400 {
            files.push(resize_image(&img, 400, &filename, &directory)?);
        }

        if width > 800 {
            files.push(resize_image(&img, 800, &filename, &directory)?);
        }

        if width > 1200 {
            files.push(resize_image(&img, 1200, &filename, &directory)?);
        }

        let path_full = image_path(&filename, "full", &directory, true);
        info!("Saving image: {:?}", path_full);
        img.save(&path_full)?;
        files.push((path_full, width, height));

        Ok(ProcessResponse { files })
    }
}

pub struct ProcessImage(pub models::File);

impl Message for ProcessImage {
    type Result = Result<ProcessResponse, DropmuttError>;
}

pub type PreFile = (PathBuf, i32, i32);

pub struct ProcessResponse {
    pub files: Vec<PreFile>,
}
