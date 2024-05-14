use clap::Parser;
use glob::glob;
use image::GenericImageView;
use rayon::prelude::*;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Parser)]
#[command(version, about = "A simple image cropping tool")]
struct CliOptions {
    /// Input image file or directory path.
    #[arg(long, short)]
    input_path: PathBuf,

    /// Output directory path.
    #[arg(long, short)]
    output_path: Option<PathBuf>,

    /// Number of threads to use.
    #[arg(long, short, default_value_t = num_cpus::get())]
    num_threads: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli_options = CliOptions::parse();

    rayon::ThreadPoolBuilder::new()
        .num_threads(cli_options.num_threads)
        .build_global()?;

    let output_path = cli_options.output_path.unwrap_or_else(|| {
        let mut default_output = cli_options.input_path.clone();
        default_output.push("output");
        std::fs::create_dir_all(&default_output).expect("Failed to create output directory");
        default_output
    });

    if cli_options.input_path.is_dir() {
        process_directory(&cli_options.input_path, &output_path)?;
    } else {
        process_file(&cli_options.input_path, &output_path)?;
    }

    Ok(())
}

fn process_directory(input_dir: &Path, output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let pattern = input_dir.join("*.png"); // Adjust pattern for different image formats if necessary
    let output_dir = Arc::new(output_dir.to_path_buf());
    glob(pattern.to_str().unwrap())?
        .filter_map(Result::ok)
        .par_bridge()
        .for_each(|path| {
            if let Err(e) = process_file(&path, &output_dir) {
                eprintln!("Failed to process file {}: {}", path.display(), e);
            }
        });
    Ok(())
}

fn process_file(input_file: &Path, output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let img = image::open(input_file)?;
    let cropped_img = crop_transparent_edges(&img);
    let aspect_corrected_img = crop_to_aspect_ratio(cropped_img);

    let file_name = input_file.file_stem().unwrap().to_str().unwrap();
    let output_file = output_dir.join(format!("{}_cropped.png", file_name));
    aspect_corrected_img.save(output_file)?;

    Ok(())
}

fn crop_transparent_edges(img: &image::DynamicImage) -> image::DynamicImage {
    let (width, height) = img.dimensions();
    let mut top = 0;
    let mut bottom = height;
    let mut left = 0;
    let mut right = width;

    'outer: for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            if pixel[3] != 0 {
                top = y;
                break 'outer;
            }
        }
    }

    'outer: for y in (0..height).rev() {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            if pixel[3] != 0 {
                bottom = y + 1;
                break 'outer;
            }
        }
    }

    'outer: for x in 0..width {
        for y in top..bottom {
            let pixel = img.get_pixel(x, y);
            if pixel[3] != 0 {
                left = x;
                break 'outer;
            }
        }
    }

    'outer: for x in (0..width).rev() {
        for y in top..bottom {
            let pixel = img.get_pixel(x, y);
            if pixel[3] != 0 {
                right = x + 1;
                break 'outer;
            }
        }
    }

    img.crop_imm(left, top, right - left, bottom - top)
}

fn crop_to_aspect_ratio(img: image::DynamicImage) -> image::DynamicImage {
    let (width, height) = img.dimensions();
    let aspect_ratio = width as f32 / height as f32;
    let min_aspect = 2.0 / 5.0; // 最小アスペクト比 2:5
    let max_aspect = 5.0 / 2.0; // 最大アスペクト比 5:2

    if aspect_ratio < min_aspect {
        // アスペクト比が小さい場合、高さを維持して幅を調整
        let new_width = (height as f32 * min_aspect) as u32;
        let new_left = (width - new_width) / 2;
        img.crop_imm(new_left, 0, new_width, height)
    } else if aspect_ratio > max_aspect {
        // アスペクト比が大きい場合、幅を維持して高さを調整
        let new_height = (width as f32 / max_aspect) as u32;
        let new_top = (height - new_height) / 2;
        img.crop_imm(0, new_top, width, new_height)
    } else {
        img
    }
}
