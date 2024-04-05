#![allow(unused_imports)]
#![allow(dead_code)]

use std::env;
use std::io::BufReader;
use std::os::unix::prelude::OsStringExt;
use std::path::PathBuf;
use std::usize::MAX;
use opencv::prelude::ColorTraitConst;
use platform_dirs::AppDirs;
use scraper::Html;
use scraper::Selector;
use regex::Regex;
use std::io::{BufWriter, Write};
use std::fs::File;
use std::io::prelude::*;
use itertools::Itertools;
use crate::Scene;
use crate::Instructional;
use crate::time_to_seconds;
use crate::get_cache_dir;
use crate::clean_title;


#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Products {
    pub products: Vec<Product>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Product {
    pub id: usize,
    pub vendor: String,
    pub title: String,
    pub handle: String
}

pub fn get_popular_creators() -> Vec<String> {
    return vec!["John Danaher", "Gordon Ryan", "Craig Jones", "Lachlan Giles", "Mikey Musumeci", "Marcelo Garcia", "Bernando Faria", "Marcus Buchecha Almeida", "Andre Galvao"].iter().map(|s| s.to_string()).collect();
}

static BLANK: &str = "";

pub fn get_cached_creators() -> Vec<String> {
    let fanatics_dir = get_cache_dir().join("bjj-fanatics");
    let path = fanatics_dir.join("products.json");
    std::fs::create_dir_all(&fanatics_dir).unwrap();
    // Try to fetch data from cache.
    if path.exists() {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let data: Products = serde_json::from_reader(reader).unwrap();
        return data.products.into_iter()
        .map(|p| p.vendor)
        .sorted()
        .dedup()
        .collect();
    }
    return get_popular_creators();
}

pub fn scrape_url(url: String) -> String {
   let id = url.split("/").last().unwrap();
   let fanatics_dir = get_cache_dir().join("bjj-fanatics");
   let path = fanatics_dir.join(id);
   std::fs::create_dir_all(fanatics_dir).unwrap();

    if path.exists() {
        let mut f = File::open(path).expect("Failed to open timstamps file from cache!");
        let mut content = String::new();
        f.read_to_string(&mut content).expect("Failed to read file!");
        return scrape_html(content);
    }

   let client = reqwest::blocking::Client::builder().cookie_store(false).build().ok().unwrap();
    let response = client.get(url)
        .header("user-agent", "rust")
        .header("accept", "*/*")
        .send()
        .ok()
        .map(|r| r.text().ok().unwrap_or_default())
        .unwrap_or_default();

    let mut f = File::create(path).expect("Failed to open timstamps file from cache!");
    f.write_all(&response.as_bytes()).expect("Failed to write timepstamps to cache!");
    scrape_html(response)
}

pub fn scrape_html(body: String) -> String {
    let html = Html::parse_document(&body);
    let selector = &Selector::parse("section table").expect("Error during the parsing using the given selector");
   
    //Get the content line by line
    let course_content = html.select(selector)
        .flat_map(|el| el.text())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    return course_content.join("\n");
}

pub fn extract_timestamps(body: String) -> Vec<Vec<Scene>> {
    let duration_re = Regex::new(r"^([0-9:]+) - ([0-9:]+)$").unwrap();
    let time_re = Regex::new(r"^([0-9:]+)$").unwrap();

    let course_content = body.lines() 
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
 
    let titles = &course_content.clone().into_iter()
        .filter(|t| !time_re.is_match(t) && !duration_re.is_match(t))
        .collect::<Vec<String>>();

    println!("Titles");
    titles.clone().into_iter().for_each(|t| println!("{}", t));

    let timestamps = course_content.clone().into_iter()
        .filter(|t| time_re.is_match(t))
        .map(|t| time_to_seconds(&t))
        .collect::<Vec<usize>>();

    let mut durations = course_content.clone().into_iter()
        .filter(|t| duration_re.is_match(t))
        .map(|t| t.split(" - ").map(|s| s.to_string()).collect::<Vec<String>>())
        .map(|a| (time_to_seconds(&a[0].to_string()), time_to_seconds(&a[1].to_string())))
        .collect::<Vec<(usize, usize)>>();

    if durations.len() == 0 {
        durations = timestamps.clone().into_iter().zip(timestamps.clone().into_iter().skip(1)).collect::<Vec<(usize, usize)>>();
    }
    
    println!("Durations");
    durations.clone().into_iter().for_each(|(s,e)| println!("{}-{}", s,e));
    // Zip titles and durations into a tuple: (title, (start, end)) 
    let all_scenes = titles.clone().into_iter().zip(durations.clone().into_iter())
        .enumerate() 
        .map(|(index, (title, (start, end)))| Scene{index, title, text: "".to_string(), start, end, labels: vec![], file: "".to_string()})
        .collect::<Vec<Scene>>();

    // Split the vector into a vector of vectors each time `end` is 0.
  // let result = all_scenes
  //       .split(|s| (s.start == 0 && s.index != 0) || s.end == 0)
  //       .map(|v| v.to_vec().into_iter().enumerate().map(|(index, s)| Scene{index, title: s.title, start: s.start, end: s.end, labels: s.labels, file: s.file}).collect_vec())
  //       .collect::<Vec<Vec<Scene>>>();

    let mut result:Vec<Vec<Scene>> = vec![];
    let mut v: i32 = -1;
    let mut index = 0;
    let mut last_start = MAX;

    println!("All scenes");
    all_scenes.into_iter().for_each(|s|  {
        // New volume
        if s.start < last_start {
            result.push(vec![]);
            v+=1;
            index=0;
        }
        last_start = s.start;
        let clean_title = clean_title(s.title);
        result[v as usize].push(Scene{index, title: clean_title, text: "".to_string(), start: s.start, end: s.end, labels: s.labels, file: s.file});
        println!("{} - {}: {}" , result[v as usize][index].start, result[v as usize][index].end, result[v as usize][index].title);
        index+=1;
    });

    println!("Scrapped:");
    for i in 0..result.len() {
        println!("Volume: {}" , (i+1));
        for j in 0..result[i].len() {
            println!("{} - {}: {}" , result[i][j].start, result[i][j].end, result[i][j].title);
        }
    }

    return result;
}

//Reoder the two strings left & right so that they match [title] - [timestamps]
pub fn check_order_s(left: &str, right: &str) -> (String, String) {
 check_order(left.to_string(), right.to_string())
}

//Reoder the two strings left & right so that they match [title] - [timestamps]
pub fn check_order(left: String, right: String) -> (String, String) {
    let time_re = Regex::new(r"^([0-9:]+)([ -]+([0-9:]+))?$").unwrap();
    if time_re.is_match(&left) {
        return (right, left);
    } else if time_re.is_match(&right) {
        return (left, right);
    } else if left.is_empty() {
        return (right, "0".to_string())
    } else if right.is_empty() {
        return (left, "0".to_string())
    } else {
        return ("".to_string(), "0".to_string())
    }
}

pub fn update_cache(creator: String, title: String)   {
    let fanatics_dir = get_cache_dir().join("bjj-fanatics");
    let path = fanatics_dir.join("products.json");
    std::fs::create_dir_all(&fanatics_dir).unwrap();

    // Try to fetch data from cache.
    if path.exists() {
       std::fs::remove_file(path).unwrap();
    }

    search_product(creator, title);
}

pub fn search_product(creator: String, title: String) -> Vec<Instructional> {
    let fanatics_dir = get_cache_dir().join("bjj-fanatics");
    let path = fanatics_dir.join("products.json");
    std::fs::create_dir_all(&fanatics_dir).unwrap();

    // Try to fetch data from cache.
    if path.exists() {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let data: Products = serde_json::from_reader(reader).unwrap();
        return data.products.into_iter()
        .filter(|p| p.title.contains(creator.as_str()) && p.title.contains(title.as_str()))
        .map(|p| product_to_instructional(p))
        .collect()
    }

    let mut result: Vec<Product> = Vec::new();
    let mut page = 1;
    let mut page_data: Vec<Product> = search_product_page(page);

    while !page_data.is_empty() {
        page_data.into_iter()
            .for_each(|i| result.push(i));
        page_data = search_product_page(page);
        page+=1;
    }

    if result.is_empty() {
        return vec![];
    }

    let products = Products{products: result.to_vec()};
    let file = File::create(path).unwrap();
    let mut out = BufWriter::new(file);
    out.write_all(serde_json::to_string(&products).expect("Failed to serialize products!").as_bytes()).expect("Failed to write products to file!");

    result.into_iter()
        .filter(|p| p.title.contains(creator.as_str()) && p.title.contains(title.as_str()))
        .map(|p| product_to_instructional(p))
        .collect()
}

pub fn product_to_instructional(product: Product) -> Instructional {
    return Instructional {creator: product.vendor, title: product.title, url: format!("https://bjjfanatics.com/products/{}", product.handle), timestamps: BLANK.to_owned(), videos: vec![] };
}

pub fn search_product_page(page: usize) -> Vec<Product> {
    let url = format!("https://bjjfanatics.com/products.json?limit=100&page={}", page).to_string();
    println!("Searching: {}", url);
    let client = reqwest::blocking::Client::builder()
        .cookie_store(false)
        .build()
        .ok()
        .unwrap();
   let response: String = client.get(url)
        .header("user-agent", "rust")
        .header("accept", "*/*")
        .send()
        .ok()
        .map(|r| r.text().ok().unwrap_or_default())
        .unwrap_or_default();

    println!("Reponse:{}", &response);
    if response.is_empty() {
        return vec![];
    }
    let data: Products = serde_json::from_str(&response).unwrap();
    let products = &data.products;

    products.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_reorder() {
        assert_eq!(("scene 1".to_string(), "00:00:00".to_string()), check_order_s("scene 1", "00:00:00"));
        assert_eq!(("scene 1".to_string(), "00:00:00".to_string()), check_order_s("00:00:00", "scene 1"));

        assert_eq!(("scene 1".to_string(), "00:00:00 - 00:10:00".to_string()), check_order_s("scene 1", "00:00:00 - 00:10:00"));
        assert_eq!(("scene 1".to_string(), "00:00:00 - 00:10:10".to_string()), check_order_s("00:00:00 - 00:10:10", "scene 1"));


        assert_eq!(("scene 1".to_string(), "00:00 - 10:00".to_string()), check_order_s("scene 1", "00:00 - 10:00"));
        assert_eq!(("scene 1".to_string(), "00:00 - 10:10".to_string()), check_order_s("00:00 - 10:10", "scene 1"));
    }
}
