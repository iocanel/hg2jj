#![allow(unused_imports)]
#![allow(dead_code)]
mod app;
mod fanatics;
use opencv::core::{bitwise_not, BORDER_CONSTANT, Size_, NORM_L1};
use opencv::photo::{fast_nl_means_denoising_vec};
use platform_dirs::AppDirs;
use regex::Regex;
use spellcheck::Speller;
use tesseract::{Tesseract, OcrEngineMode};
use std::collections::HashMap;
use std::env;
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::io::{BufWriter, Write};
use std::fs::File;
use std::io::prelude::*;
use ffprobe::*;
use itertools::Itertools;
pub use app::App;
pub use fanatics::*;

use opencv::{
    imgcodecs::*,
    imgproc::*,
    types::*,
    prelude::*,
};

#[derive(Debug, Clone)]
pub struct Instructional {
    creator: String,
    title: String,
    url: String,
    timestamps: String,
    videos: Vec<Video>
}

#[derive(Debug, Clone)]
pub struct Video {
    index: usize,
    file: String,
    duration: usize,
    scenes: Vec<Scene>,
}

#[derive(Debug, Clone)]
pub struct Scene {
    index: usize,
    title: String,
    file: String,
    labels: Vec<String>,
    start: usize,
    end: usize
}

#[derive(Debug, Clone, Copy)]
pub struct DetectionSettings {
    threshold: f32,
    minimum_length: i32
}

impl DetectionSettings {
    fn new() -> Self {
        DetectionSettings {
            threshold: 0.2,
            minimum_length: 10
        }
    }
}

impl Default for DetectionSettings {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct GeneralSettings {
    step_in_secs: usize,
    org_export_enabled: bool,
    org_export_filename: String,
    md_export_enabled: bool,
    md_export_filename: String,
    playlist_export_enabled: bool,
    playlist_export_filename: String
}

impl GeneralSettings {
    fn new() -> Self {
        GeneralSettings {
            step_in_secs: 1,
            org_export_enabled: true,
            org_export_filename: "readme.org".to_string(),
            md_export_enabled: true,
            md_export_filename: "readme.md".to_string(),
            playlist_export_enabled: true,
            playlist_export_filename: "playlist.m3u".to_string()
        }
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Case {
    Upper,
    Lower,
    CapitalizeFirst,
}

#[derive(Debug, Clone)]
pub struct OcrSettings {
    grayscale: bool,

    gaussian_thresholding: bool,
    gaussian_thresholding_max_value: f64,
    gaussian_thresholding_blocksize: i32,
    gaussian_thresholding_c: f64,

    otsu_thresholding: bool,
    otsu_thresholding_min_value: f64,
    otsu_thresholding_max_value: f64,

    denoise: bool,
    denoise_strength: f32,

    erode: bool,
    erode_kernel_size: i32,
    erode_iterations: i32,
    dilate: bool,
    dilate_kernel_size: i32,
    dilate_iterations: i32,

    invert: bool,
    spellcheking: bool,
    case: Case

}
impl OcrSettings {
    fn new() -> Self {
        OcrSettings {
            grayscale: true,
            invert: true,
            gaussian_thresholding: false,
            gaussian_thresholding_max_value: 255.0,
            gaussian_thresholding_blocksize: 11,
            gaussian_thresholding_c: 2.0,
            otsu_thresholding: false,
            otsu_thresholding_min_value: 0.0,
            otsu_thresholding_max_value: 255.0,
            denoise: true,
            denoise_strength: 3.0,
            erode: true,
            erode_kernel_size:3,
            erode_iterations:1,
            dilate: true,
            dilate_kernel_size:3,
            dilate_iterations:1,
            spellcheking: true,
            case: Case::CapitalizeFirst
        }
    }
}

static BLANK: &str = "";

pub fn load_org(mut f: File) -> Instructional {
    let mut content = String::new();
    f.read_to_string(&mut content).expect("Failed to read file!");
    parse_org(content)
}

pub fn parse_org(content: String) -> Instructional {
    let lines: Vec<String> = content.lines().map(|i| i.to_string()).collect();
    let mut scenes: Vec<Scene> = Vec::new();

    let title_re = Regex::new(r"#\+title: (.*)").unwrap();
    let creator_re = Regex::new(r"#\+creator: (.*)").unwrap();
    let url_re = Regex::new(r"#\+url: (.*)").unwrap();

    let volume_re = Regex::new(r"^\*+ Volume [0-9]+$").unwrap();
    let s_title_re = Regex::new(r"^\*+ ([a-zA-Z0-9'`\.,_ /&:-]+) (:[a-zA-Z0-9_-]+:)$").unwrap();
    let start_timestamp_re = Regex::new(r":START_TIMESTAMP:[ ]+([0-9]+)").unwrap();
    let end_timestamp_re = Regex::new(r":END_TIMESTAMP:[ ]*([0-9]+)").unwrap();
    let end_re = Regex::new(r":END:").unwrap();
    let file_re = Regex::new(r":FILE_OR_URL:(.+)$").unwrap();

    let duration_re = Regex::new(r":DURATION:[ ]*([0-9]+)").unwrap();

    let mut creator=String::from("unknown");
    let mut title=String::from("unknown");
    let mut url=String::from("");
    
    //Scene 
    let mut index: usize = 1;
    let mut s_title = String::new();
    let mut file = String::new();
    let mut labels: Vec<String> = Vec::new();
    let mut start: Option<usize> = None;
    let mut end: Option<usize> = None;

    for line in lines {
        if creator_re.is_match(&line) {
            let cap = creator_re.captures(&line).expect("Failed to match regex!");
            creator = cap.get(1).map(|m| m.as_str().to_string()).expect("Failed to caputre creator!");
        }

        if title_re.is_match(&line) {
            let cap = title_re.captures(&line).expect("Failed to match regex!");
            title = cap.get(1).map(|m| m.as_str().to_string()).expect("Failed to caputre title!");
        }

        if url_re.is_match(&line) {
            let cap = url_re.captures(&line).expect("Failed to match regex!");
            url = cap.get(1).map(|m| m.as_str().to_string()).expect("Failed to caputre url!");
        }

        if volume_re.is_match(&line) {
            start = None;
            end = None;
            index = 0;
            s_title = String::new();
            file = String::new();
        }

        if s_title_re.is_match(&line) {
            let cap = s_title_re.captures(&line).expect("Failed to match regex!");
            s_title = clean_title(cap.get(1).map(|m| m.as_str().to_string()).expect("Failed to caputre title!"));
        }
        if start_timestamp_re.is_match(&line) {
            let cap = start_timestamp_re.captures(&line).expect("Failed to match regex!");
            start = cap.get(1).map(|m| m.as_str().parse::<usize>().expect("Failed to parse start timestamp!"));
        }
        if end_timestamp_re.is_match(&line) {
            let cap = end_timestamp_re.captures(&line).expect("Failed to match regex!");
            end = cap.get(1).map(|m| m.as_str().parse::<usize>().expect("Failed to parse end timestamp!"));
        }
        if file_re.is_match(&line) {
            let cap = file_re.captures(&line).expect("Failed to match regex!");
            file = cap.get(1)
                .map(|m| m.as_str().trim().to_string())
                .expect("Failed to capture end file!");
        }

        //When we reach properties end we push the scene
        if start.is_some() && end.is_some() && !s_title.is_empty() && !file.is_empty() {
                scenes.push(Scene {
                    index,
                    title: s_title,
                    labels,
                    file,
                    start: start.unwrap_or(0),
                    end: end.unwrap_or(0)});

                s_title = String::new();
                file = String::new();
                index += 1;
                file = String::new();
                labels = vec![];
                start = None;
                end = None;
        }
    }

    //Sort scenes by file
    scenes.sort_by(|a, b| a.file.partial_cmp(&b.file).unwrap());

   let videos: Vec<Video> = scenes.iter()
        .group_by(|s| &s.file)
        .into_iter()
        .enumerate()
        .map(|(i, (f, s))| Video{index: i,
                                 file: f.to_string(),
                                 duration: 0,
                                 scenes: s.into_iter().cloned().collect()})
        .map(|mut v| {
            let scenes = &v.scenes;
            v.duration = scenes.into_iter().map(|s| s.end).reduce(|a, b| if a > b { a } else { b  } ).unwrap_or_default();
           return v;
        }).collect();
    
   Instructional{creator, title, url, timestamps: BLANK.to_owned(), videos}
}

pub fn save_playlist(instructional: &mut Instructional, out: File) {
    let mut out = BufWriter::new(out);
    out.write_all(format!("#EXTM3U\n#EXT-X-VERSION:6\n").as_bytes()).expect("Unable to write playlist header!");
    instructional.videos.iter().for_each(|v| {
        v.scenes.iter().for_each(|s| {
            let file_name = PathBuf::from(s.file.clone()).file_name().expect("Unable to get filename!").to_os_string().into_string().expect("Cannot convert into string");
            out.write_all(format!("#EXTINF: {}, {}\n", s.start, s.title).as_bytes()).expect("Unable to write scene header!");
            out.write_all(format!("{}", file_name).as_bytes()).expect("Unable to write scene file!");
            out.write_all("\n".as_bytes()).expect("Unable to write separator line!");
        });
    });
}

pub fn save_org(instructional: &mut Instructional, out: File, absolute: bool) {
    let mut out = BufWriter::new(out);
    out.write_all(format!("#+creator: {}\n", instructional.creator).as_bytes()).expect("Unable to write creator!");
    out.write_all(format!("#+title: {}\n", clean_title(instructional.title.to_string())).as_bytes()).expect("Unable to write title!");
    out.write_all(format!("#+url: {}\n", instructional.url).as_bytes()).expect("Unable to write title!");
    out.write_all("\n".as_bytes()).expect("Unable to write separator line!");
    instructional.videos.iter().for_each(|v| {
        out.write_all(format!("** Volume {}\n", v.index + 1).as_bytes()).expect("Unable to write video entry!");
        if v.duration > 0 {
            out.write_all(":PROPERTIES:\n".as_bytes()).expect("Unable to write video properties start!");
            out.write_all(format!(":DURATION: {}\n", v.duration).as_bytes()).expect("Unable to write video duration!");
            out.write_all(":END:\n".as_bytes()).expect("Unable to write video properties end!");
        }
        out.write_all("\n".as_bytes()).expect("Unable to write scene properties end!");
        v.scenes.iter().for_each(|s| {
            let file_or_url = if absolute { s.file.clone() } else { PathBuf::from(s.file.clone()).file_name().expect("Unable to get filename!").to_os_string().into_string().expect("Cannot convert into string") };
            s.labels.iter().fold(String::from(":video:"), |all, l| format!("{}{}", all.chars().take(all.len()-1).collect::<String>(), l));
            out.write_all(format!("*** {} :video:\n", clean_title(s.title.to_string())).as_bytes()).expect("Unable to write scene title!");
            out.write_all(":PROPERTIES:\n".as_bytes()).expect("Unable to write scene properties start!");
            out.write_all(format!(":INDEX: {}\n", s.index + 1).as_bytes()).expect("Unable to write scene index!");
            out.write_all(format!(":FILE_OR_URL: {}\n", file_or_url).as_bytes()).expect("Unable to write scene file or url!");
            out.write_all(format!(":START_TIMESTAMP: {}\n", s.start).as_bytes()).expect("Unable to write scene start timestamp!");
            out.write_all(format!(":END_TIMESTAMP: {}\n", s.end).as_bytes()).expect("Unable to write scene end timestamp!");
            out.write_all(":END:\n".as_bytes()).expect("Unable to write scene properties end!");
            out.write_all("\n".as_bytes()).expect("Unable to write separator line!");
        });
        out.write_all("\n".as_bytes()).expect("Unable to write scene properties end!");
    });
}

pub fn save_md(instructional: &mut Instructional, out: File) {
    let mut out = BufWriter::new(out);
    instructional.videos.iter().for_each(|v| {
        let has_multiple_scenes_per_video = instructional.videos.iter().filter(|v| v.scenes.len() > 1).count() > 0;
        if has_multiple_scenes_per_video {
            out.write_all(format!("# Volume {}\n", v.index + 1).as_bytes()).expect("Unable to write video entry!");
        }
        v.scenes.iter().for_each(|s| {
            let file_name = PathBuf::from(s.file.clone()).file_name().expect("Unable to get filename!").to_os_string().into_string().expect("Cannot convert into string");
            out.write_all(format!("## {}\n", clean_title(s.title.to_string())).as_bytes()).expect("Unable to write scene title!");
            out.write_all(format!("![{}](){}\n", s.title, file_name).as_bytes()).expect("Unable to write scene file or url!");
            out.write_all("\n".as_bytes()).expect("Unable to write separator line!");
        });
        out.write_all("\n".as_bytes()).expect("Unable to write scene properties end!");
    });
}

pub fn scene_detect(path: String, settings: DetectionSettings) -> Vec<(usize, f32)> {
    let path = escape_path(path.as_str());
    let time_re = Regex::new(r".*best_effort_timestamp_time=([0-9\.]+).*scene_score=([0-9\.]+)").expect("Failed to define regular expression for timestamp in ffprobe output!");
    let cmd = if cfg!(target_os = "windows") { "ffprobe.exe" } else { "ffprobe" };
    let out = std::process::Command::new(cmd)
        .args([
            "-v",
            "quiet",
            "-show_packets",
            "-show_streams",
            "-show_format",
            "-show_frames",
            "-of",
            "compact=p=0",
            "-f",
            "lavfi",
            format!("movie={},select=gt(scene\\,{:.1})", path, settings.threshold).as_str()
        ])
        .output()
        .map_err(FfProbeError::Io).unwrap();

    let output = String::from_utf8(out.stdout).unwrap();
    if !out.status.success() {
        println!("Failed to run {}.", cmd);
        println!("{}", output);
        return vec![];
    }
    return output.as_str()
        .lines()
        .filter(|l| time_re.is_match(l))
        .map(|l| time_re.captures(l).expect("Failed to caputre timestamp!"))
        .map(|c| (c.get(1).map(|m| m.as_str().parse::<f32>().expect("Failed to parse timestamp!")).expect("Failed to match timestamp group!") as usize
                 ,c.get(2).map(|m| m.as_str().parse::<f32>().expect("Failed to parse timestamp!")).expect("Failed to match timestamp group!") * 100.0)).collect();
}

pub fn scene_img_path(creator: String, title: String, scene: &Scene) -> Option<PathBuf> {
    let path = Path::new(&scene.file);
    let file_name = path.file_name()?;
    let instructional_dir = get_cache_dir().join("instructionals").join(creator).join(title);
    std::fs::create_dir_all(&instructional_dir).expect("Failed to create instructional directory!");
    let mut img_filename = Path::new(file_name).file_stem()?.to_str()?.to_string();
    img_filename.push_str("-");
    img_filename.push_str(scene.start.to_string().as_str());
    img_filename.push_str(".png");
    Some(instructional_dir.join(img_filename))
}

pub fn scene_ocr_img_path(creator: String, title: String, scene: &Scene) -> Option<PathBuf> {
    let path = Path::new(&scene.file);
    let file_name = path.file_name()?;
    let instructional_dir = get_cache_dir().join("instructionals").join(creator).join(title);
    std::fs::create_dir_all(&instructional_dir).expect("Failed to create instructional directory!");
    let mut img_filename = Path::new(file_name).file_stem()?.to_str()?.to_string();
    img_filename.push_str("-");
    img_filename.push_str(scene.start.to_string().as_str());
    img_filename.push_str("-ocr");
    img_filename.push_str(".png");
    Some(instructional_dir.join(img_filename))
}

pub fn scene_to_image(creator: String, title: String, scene: &Scene) -> Option<String> {
    let mut img_path = scene_img_path(creator, title, scene)?;
    let img_path_str = img_path.to_str().expect("Failed to convert image output path to String!");
    if img_path.exists() {
        //A user is expected to recreate the file after tuning the offset.
        //Sicne the offset is part of the file name there is no reason to recreate the image for a specific offset (the result will be the same).
        println!("Scene image: {} found. Reusing ...", img_path_str);
        return Some(img_path_str.to_string())
    }
    println!("Scene image: {} does not exist. Creating ...", img_path_str);
    let cmd = if cfg!(target_os = "windows") { "ffmpeg.exe" } else { "ffmpeg" };
    let out = std::process::Command::new(cmd)
        .args([
            "-i",
            scene.file.as_str(),
            "-ss",
            scene.start.to_string().as_str(),
            "-dpi",
            "300",
            "-vframes",
            "1",
            img_path_str
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();

    if !out.status.success() {
        return None;
    }
    
    Some(img_path_str.to_string())
}

pub fn all_scenes(instructional: Instructional) -> Vec<Scene> {
    let mut result: Vec<Scene> = Vec::new();
    for i in 0..instructional.videos.len() {
        for j in 0..instructional.videos[i].scenes.len()  {
           result.push(instructional.videos[i].scenes[j].clone()); 
        }
    }
    return result;
}

pub fn split_scene(index: usize, s: Scene) -> Option<Video>  {
   let cmd = if cfg!(target_os = "windows") { "ffmpeg.exe" } else { "ffmpeg" };
    let extension = &s.file.split(".").last().unwrap_or("mp4");
    let path = Path::new(&s.file);
    let file = path.parent().unwrap().join(format!("{:03}. {}.{}", index, &s.title, extension).to_string()).to_str().unwrap().to_string();
    let mut args: Vec<String> = vec![
                "-i",
                s.file.as_str(),
                "-ss",
                s.start.to_string().as_str()].iter().map(|s| s.to_string()).collect();

            if s.end > 0 {
                args.push("-to".to_string());
                args.push(s.end.to_string());
            }
            args.push(file.to_string());
            std::process::Command::new(cmd)
                .args(args)
                .stdin(Stdio::null())
                .output()
                .unwrap();

           return Some(Video {index, file: file.to_string(), duration: 0, scenes: vec![Scene {index: 1, title: s.title.to_string(), file: file.to_string(), start: 0, end: 0, labels: vec![] }]});
}

pub fn play_scene(scene: Scene) {
    let cmd = if cfg!(target_os = "windows") { "mpv.exe" } else { "mpv" };
    let path = escape_path(&scene.file);
    let out = std::process::Command::new(cmd)
        .args([
            format!("--start={}", scene.start),
            path.clone()
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();

    if !out.status.success() {
        println!("Failed to start mpv for video: {}", path);
    }
}

pub fn ocr_preprocess_img(path: String, ocr_settings: &OcrSettings) -> Option<String> {
    println!("Starting OCR preprocessing: {}", path);
    let output_filename = path.replace(".png", "-ocr.png");
    println!("Ouput OCR preprocessing file: {}", output_filename);
    let src_img = imread(path.as_str(), IMREAD_COLOR).expect("Failed to load image!");
    let mut dst_img = Mat::default();
    ocr_preprocess(src_img, &mut dst_img, ocr_settings);
    imwrite(&output_filename, &dst_img, &VectorOfi32::new()).expect("Failed to write preprocessed image!");
    return Some(output_filename);
}

pub fn ocr_preprocess(src_img: Mat, dst_img: &mut Mat, settings: &OcrSettings) {
    let mut cur_img = Mat::default();
    src_img.copy_to(&mut cur_img).expect("Failed to copy image!");
    if settings.grayscale {
        cvt_color(&cur_img, dst_img, COLOR_BGRA2GRAY, 0).expect("Failed to convert image to grayscale!");
        dst_img.copy_to(&mut cur_img).expect("Failed to copy image!");
    }

    if settings.gaussian_thresholding {
        adaptive_threshold(&cur_img, dst_img, settings.gaussian_thresholding_max_value, ADAPTIVE_THRESH_GAUSSIAN_C, THRESH_BINARY, settings.gaussian_thresholding_blocksize, settings.gaussian_thresholding_c).expect("Failed to apply Gaussian thresholding!");
        dst_img.copy_to(&mut cur_img).expect("Failed to copy image!");
    }

    if settings.otsu_thresholding {
        threshold(&cur_img, dst_img, settings.otsu_thresholding_min_value, settings.otsu_thresholding_max_value, THRESH_OTSU).expect("failed to apply OTSU thresholding!");
        dst_img.copy_to(&mut cur_img).expect("Failed to copy image!");
    }

    if settings.invert {
        bitwise_not(&mut cur_img, dst_img, &Mat::default()).expect("Failed to invert image!");
        dst_img.copy_to(&mut cur_img).expect("Failed to copy image!");
    }

    if settings.denoise {
        let mut strength_vector = opencv::core::Vector::new();
        strength_vector.insert(0, settings.denoise_strength);
        fast_nl_means_denoising_vec(&mut cur_img, dst_img, &strength_vector, 7, 21, NORM_L1);
    }
//    fast_nl_means_denoising(&mut cur_img, dst_img, 3.0, 3.0, 7, 21);
    dst_img.copy_to(&mut cur_img).expect("Failed to copy image!");

    if settings.erode {
        let anchor = opencv::core::Point::new(-1, -1);
        let kernel = get_structuring_element(MORPH_RECT, Size_ { width: settings.erode_iterations, height: settings.erode_kernel_size }  , anchor).unwrap();
        match erode(&mut cur_img, dst_img, &kernel, anchor, settings.erode_iterations, BORDER_CONSTANT, morphology_default_border_value().unwrap()) {
            Ok(_) => {
                dst_img.copy_to(&mut cur_img).expect("Failed to copy image!");
            },
            Err(e) => { println!("Erosion failed:{}", e) }
            } 
    }
    if settings.dilate {
        let anchor = opencv::core::Point::new(-1, -1);
        let kernel = get_structuring_element(MORPH_RECT, Size_ { width: settings.erode_iterations, height: settings.erode_kernel_size }  , anchor).unwrap();
        match dilate(&mut cur_img, dst_img, &kernel, anchor, settings.dilate_iterations, BORDER_CONSTANT, morphology_default_border_value().unwrap()) {
            Ok(_) => {
                dst_img.copy_to(&mut cur_img).expect("Failed to copy image!");
            },
            Err(e) => { println!("Erosion failed:{}", e) }
        } 
    }
}

pub fn scene_text_with_settings(creator: String, title: String, scene: &Scene, ocr_settings: &OcrSettings) -> Option<String> {
    if let Some(img_file) = scene_to_image(creator, title, scene) {
        if let Some(ocr_file) = ocr_preprocess_img(img_file, ocr_settings) {
            let tesseract =  Tesseract::new_with_oem(None, Some("eng"), OcrEngineMode::LstmOnly).expect("Failed to initialize tesseract!");
            let mut recongnize = tesseract.set_image(ocr_file.as_str()).expect("Failed to set image!").recognize().expect("Failed to recognize text!");
            let alpha_re = Regex::new(r"[^a-zA-Z]+$").unwrap();
            let space_or_newline_re = Regex::new(r"[\n\r\s]+").unwrap();
            let invalid_characters_re = Regex::new(r"[^a-zA-Z0-9\\(\\) -]").unwrap();
            let ocr_text = recongnize.get_text().expect("Failed to get text from tesseract!");
            let text_single_line = space_or_newline_re.replace_all(&ocr_text, " ");
            let text = invalid_characters_re.replace(&text_single_line, "").to_string();

            println!("text:{}", text);
            if ocr_settings.spellcheking { 
                let mut speller = Speller {
                    letters: "abcdefghijklmnopqrstuvwxyz".to_string(),
                    n_words: HashMap::new()
                };
                let training_data = std::fs::read_to_string("assets/dict/rgjj.txt").expect("Failed to read spellchecking dictionary");
                speller.train(&training_data);

                return Some(text.trim().split(" ")
                            .map(|w| clean_title(w.to_string()))
                            .map(|w| if !w.is_empty() && alpha_re.is_match(&w) { speller.correct(&w) } else { w.to_string() })
                            .map(|w| apply_case(w, ocr_settings.case))
                            .intersperse(" ".to_string())
                            .collect());
            } else {
                return Some(text.trim().split(" ")
                            .map(|w| clean_title(w.to_string()))
                            .map(|w| apply_case(w, ocr_settings.case))
                            .intersperse(" ".to_string())
                            .collect());
            }
        }
    }
    return None;
}

pub fn scene_text(creator: String, title: String, scene: &Scene) -> Option<String> {
    scene_text_with_settings(creator, title, scene, &OcrSettings::new())
}

fn apply_case(s: String, case: Case) -> String {
    return match case {
       Case::Lower => s.to_lowercase().to_string(),
       Case::Upper => s.to_uppercase().to_string(),
       Case::CapitalizeFirst => capitalize_first(s)
    }
}

fn capitalize_first(s: String) -> String {
   return s.chars().enumerate().map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c.to_ascii_lowercase() }).collect();
}

pub fn video_duration(p: String) -> usize {
    let file = Path::new(&p);
    match ffmpeg::format::input(&file) {
	Ok(context) => {
            if let Some(stream) = context.streams().best(ffmpeg::media::Type::Video) {
                let result = stream.duration() as i64 * stream.time_base().0 as i64 / stream.time_base().1 as i64;
                return result as usize;
	    }
        }, Err(error) => println!("Error: {}", error)
    }
    0
}

pub fn seconds_to_time(seconds: usize) -> String {
    let hours = seconds / 3600;
    let hours_r = seconds % 3600;
    let minutes = hours_r / 60;
    let minutes_r = hours_r % 60;
    // It's highly unlikely an instructional video to be longer than 3 hours.
    // So we assume that this is an error.
    if hours > 3 {
        return format!("{:02}:{:02}:{:02}", 0 , hours, minutes);
    }
    return format!("{:02}:{:02}:{:02}", hours, minutes, minutes_r);
}

pub fn time_to_seconds(time: &str) -> usize {
   let mut parts: Vec<u32> = time.split(":").map(|s| s.parse::<u32>().unwrap_or(0)).collect();
    // It's highly unlikely an instructional video to be longer than 3 hours.
    // So we assume that this is an error.
    let last_index = parts.len() - 1;
    if parts.len() >= 3 && parts[0] > 3 && parts[last_index] == 0 {
        // Let's remove the last element (skip all parts to the right).
        parts = parts.into_iter().rev().skip(1).rev().collect();
    }
    parts.into_iter()
        .rev()
        .enumerate()
        .map(|(i, t)| (60 as u32).pow(i as u32) * t)
        .reduce(|a, b| a + b)
        .unwrap() as usize
}

pub fn get_cache_dir() -> PathBuf {
    return match env::var("HG2JJ_DIR") {
        Ok(d) => PathBuf::from(d).join(".cache"),
        Err(_) => AppDirs::new(Some("hg2jj"), false).map(|d| d.cache_dir).unwrap(),
    };
}

pub fn escape_path(path: &str) -> String {
    path.to_string()
        .replace("&","\\&")
        .replace("[","\\[")
        .replace("]","\\]")
}

pub fn clean_title(title: String) -> String {
    let multi_space_re = Regex::new(r"[ \t]{2,}").unwrap();
    let invalid_characters_re = Regex::new(r"[^a-zA-Z0-9&\\(\\) -]").unwrap();
    return invalid_characters_re.replace_all(&multi_space_re.replace_all(title.as_str(), " ").replace("/", "-"), "").to_string();
}
// ----------------------------------------------------------------------------
// When compiling for web:

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};

/// This is the entry-point for all the web-assembly.
/// This is called once from the HTML.
/// It loads the app, installs some callbacks, then returns.
/// You can add more callbacks like this if you want to call in to your code.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), eframe::wasm_bindgen::JsValue> {
    let app = App::default();
    eframe::start_web(canvas_id, Box::new(app))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_org() {
        let mut content = String::from("
#+creator: iocanel
#+title: my test

*** Scene 1 :video:
:PROPERTIES::
:FILE_OR_URL: vol1.mp4
:START_TIMESTAMP: 0
:END_TIMESTAMP: 100
:END:

Random comments

* Scene 2 :video:
:PROPERTIES:
:FILE_OR_URL: vol1.mp4
:START_TIMESTAMP: 100
:END_TIMESTAMP: 200
:END:
");
        let i = parse_org(content);
        assert_eq!("iocanel", i.creator);
        assert_eq!("my test", i.title);
        assert_eq!(1, i.videos.len());
        assert_eq!(2, i.videos[0].scenes.len());
        assert_eq!("Scene 1", i.videos[0].scenes[0].title);
        assert_eq!("vol1.mp4", i.videos[0].scenes[0].file);
        assert_eq!(0, i.videos[0].scenes[0].start);
        assert_eq!(100, i.videos[0].scenes[0].end);
        assert_eq!("Scene 2", i.videos[0].scenes[1].title);
        assert_eq!("vol1.mp4", i.videos[0].scenes[1].file);
        assert_eq!(100, i.videos[0].scenes[1].start);
        assert_eq!(200, i.videos[0].scenes[1].end);
    }


    #[test]
    fn test_time_to_seconds() {
        assert_eq!(0, time_to_seconds("0"));
        assert_eq!(0, time_to_seconds("00:00:00"));
        assert_eq!(1, time_to_seconds("01"));
        assert_eq!(1, time_to_seconds("00:00:01"));
        assert_eq!(60, time_to_seconds("01:00"));
        assert_eq!(60, time_to_seconds("00:01:00"));
        assert_eq!(3600, time_to_seconds("01:00:00"));
        assert_eq!(3601, time_to_seconds("01:00:01"));
        assert_eq!(3661, time_to_seconds("01:01:01"));

        // actual probelms
        assert_eq!(1380, time_to_seconds("23:00"));
        assert_eq!(1380, time_to_seconds("23:00:00"));
        assert_eq!(73, time_to_seconds("1:13"));
        assert_eq!(4690, time_to_seconds("1:18:10"));
    }

    #[test]
    fn test_seconds_to_time() {
        assert_eq!("00:00:01".to_string(), seconds_to_time(1));
        assert_eq!("00:01:00".to_string(), seconds_to_time(60));
        assert_eq!("01:00:00".to_string(), seconds_to_time(3600));
        assert_eq!("01:00:01".to_string(), seconds_to_time(3601));
        assert_eq!("01:01:01".to_string(), seconds_to_time(3661));
    }

    #[test]
    fn test_clean_title() {
        assert_eq!("A B".to_string(), clean_title("A  B".to_string()));
        assert_eq!("A B".to_string(), clean_title("A   B".to_string()));
        assert_eq!("A B".to_string(), clean_title("A    B".to_string()));

        assert_eq!("A-B".to_string(), clean_title("A/B".to_string()));
        assert_eq!("A-B-C".to_string(), clean_title("A/B/C".to_string()));

        assert_eq!("AB".to_string(), clean_title("A'B".to_string()));
        assert_eq!("AB".to_string(), clean_title("A*B".to_string()));
        assert_eq!("A&B".to_string(), clean_title("A&B".to_string()));
    }
}
