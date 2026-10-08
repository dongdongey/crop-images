use image::{DynamicImage, GenericImageView};
use rayon::prelude::*;
use std::cmp::{max, min};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// (x, y)부터 (z, w) 좌표 영역을 안전하게 크롭합니다.
pub fn crop_by_coordinates(
    img: &DynamicImage,
    x: u32,
    y: u32,
    z: u32,
    w: u32,
) -> Option<DynamicImage> {
    let (img_w, img_h) = img.dimensions();

    let start_x = min(x, z).min(img_w);
    let start_y = min(y, w).min(img_h);
    let end_x = max(x, z).min(img_w);
    let end_y = max(y, w).min(img_h);

    let crop_w = end_x.saturating_sub(start_x);
    let crop_h = end_y.saturating_sub(start_y);

    if crop_w == 0 || crop_h == 0 {
        return None;
    }

    Some(img.crop_imm(start_x, start_y, crop_w, crop_h))
}

fn process_image(
    input_path: &Path,
    output_dir: &Path,
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let img = image::open(input_path)?;
    let cropped = crop_by_coordinates(&img, x1, y1, x2, y2)
        .ok_or("크롭 영역의 크기가 0이거나 이미지 범위를 벗어났습니다.")?;
    let file_name = input_path
        .file_name()
        .ok_or("유효하지 않은 파일 이름입니다.")?;
    let output_path = output_dir.join(file_name);

    cropped.save(&output_path)?;
    Ok(())
}

#[inline]
fn is_supported_image(path: &Path) -> bool {
    image::ImageFormat::from_path(path).is_ok()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();

    if args.len() < 5 {
        println!(
            "사용법: {} <x1> <y1> <x2> <y2> [입력_디렉토리] [출력_디렉토리]",
            args[0]
        );
        std::process::exit(1);
    }
    let crop_points: Vec<u32> = args[1..5]
        .iter()
        .map(|arg| arg.parse::<u32>())
        .collect::<Result<_, _>>()
        .map_err(|_| "x1, y1, x2, y2 좌표는 0 이상의 정수(u32)여야 합니다.")?;

    let input_dir = if let Some(target) = args.get(5) {
        PathBuf::from(target)
    } else {
        std::env::current_dir()?
    };

    let output_dir = if let Some(out) = args.get(6) {
        PathBuf::from(out)
    } else {
        input_dir.join("cropped")
    };
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir)?;
    }

    let file_paths: Vec<PathBuf> = std::fs::read_dir(&input_dir)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file() && is_supported_image(path))
        .collect();

    if file_paths.is_empty() {
        println!("처리할 이미지 파일이 없습니다: {:?}", input_dir);
        return Ok(());
    }

    println!("총 {}장의 이미지를 병렬로 처리합니다...", file_paths.len());

    let (x, y, z, w) = (
        crop_points[0],
        crop_points[1],
        crop_points[2],
        crop_points[3],
    );

    let success_count = AtomicUsize::new(0);
    let fail_count = AtomicUsize::new(0);

    file_paths
        .par_iter()
        .for_each(|path| match process_image(path, &output_dir, x, y, z, w) {
            Ok(_) => {
                success_count.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                fail_count.fetch_add(1, Ordering::Relaxed);
                eprintln!("[실패] {:?}: {}", path.file_name().unwrap_or_default(), e)
            }
        });
    println!(
        "\n처리 완료 --> 성공: {}장, 실패: {}장",
        success_count.load(Ordering::Relaxed),
        fail_count.load(Ordering::Relaxed)
    );
    println!("결과 저장 경로: {:?}", output_dir);
    Ok(())
}
