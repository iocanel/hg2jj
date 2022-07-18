#![allow(unused_imports)]
#![allow(dead_code)]

use std::env;
use std::io::BufReader;
use std::os::unix::prelude::OsStringExt;
use std::path::PathBuf;
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

pub fn scrape_url(url: String) -> Vec<Vec<Scene>> {
   let id = url.split("/").last().unwrap();
   let fanatics_dir = get_cache_dir().join("bjj-fanatics");
   let path = fanatics_dir.join(id);
   std::fs::create_dir_all(fanatics_dir).unwrap();

    if path.exists() {
        let mut f = File::open(path).expect("Failed to open timstamps file from cache!");
        let mut content = String::new();
        f.read_to_string(&mut content).expect("Failed to read file!");
        return scrape_response(content);
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
    scrape_response(response)
}

pub fn scrape_response(body: String) -> Vec<Vec<Scene>> {
    let html = Html::parse_document(&body);
    let selector = &Selector::parse("section#contents table").expect("Error during the parsing using the given selector");
    let volume_re = Regex::new(r"^Volume ([0-9]+)").unwrap();
    let time_re = Regex::new(r"^([0-9:]+)([ -]+([0-9:]+))?$").unwrap();
    let duration_re = Regex::new(r"^([0-9:]+) - ([0-9:]+)$").unwrap();
    let time_re = Regex::new(r"^([0-9:]+)$").unwrap();
    
    let mut result: Vec<Vec<Scene>> = Vec::new();

    //Get the content line by line
    let course_content = html.select(selector)
        .flat_map(|el| el.text())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .skip(1) // Skip 'Course Content' heading
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    let titles = &course_content.clone().into_iter()
        .filter(|t| !time_re.is_match(t) && !duration_re.is_match(t))
        .collect::<Vec<String>>();

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
    
    // Zip titles and durations into a tuple: (title, (start, end)) 
    let all_scenes = titles.clone().into_iter().zip(durations.clone().into_iter())
        .map(|(title, (start, end))| Scene{index: 0, title, start, end, labels: vec![], file: "".to_string()})
        .collect::<Vec<Scene>>();

    // Split the vector into a vector of vectors each time `end` is 0.
   all_scenes
        .split(|s| s.end == 0)
        .map(|v| v.to_vec().into_iter().enumerate().map(|(index, s)| Scene{index, title: s.title, start: s.start, end: s.end, labels: s.labels, file: s.file}).collect_vec())
        .collect::<Vec<Vec<Scene>>>()
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

    search_product("".to_string(), "".to_string());
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
    return Instructional {creator: product.vendor, title: product.title, url: format!("https://bjjfanatics.com/products/{}", product.handle), videos: vec![] };
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
