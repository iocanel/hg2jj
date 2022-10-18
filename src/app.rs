#![allow(unused_imports)]
#![allow(dead_code)]
use crate::DetectionSettings;
use crate::GeneralSettings;
use crate::all_scenes;
use crate::app::egui::Vec2;
use crate::extract_timestamps;
use crate::get_cached_creators;
use crate::get_popular_creators;
use crate::load_org;
use crate::play_scene;
use crate::save_org;
use crate::save_md;
use crate::save_playlist;
use crate::scene_detect;
use crate::scene_ocr_img_path;
use crate::scene_text_with_settings;
use crate::scene_to_image;
use crate::scrape_url;
use crate::search_product;
use crate::seconds_to_time;
use crate::split_scene;
use crate::update_cache;
use crate::video_duration;
use crate::File;
use crate::Instructional;
use crate::OcrSettings;
use crate::Scene;
use crate::Video;
use eframe::{egui, epi};
use egui::*;
use itertools::EitherOrBoth::Both;
use itertools::EitherOrBoth::Left;
use itertools::EitherOrBoth::Right;
use itertools::Itertools;
use platform_dirs::AppDirs;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

static BLANK: &str = "";

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct App {
    icons: HashMap<&'static str, TextureId>,
    file: String,
    last_selected_file: String,
    instructional: Instructional,
    candidate_creators: Vec<String>,
    use_creator_combo: bool,
    candidate_titles: Vec<String>,
    use_title_combo: bool,
    candidate_urls: Vec<String>,
    scene_images: Vec<Vec<Option<egui::TextureId>>>,
    general_settings: GeneralSettings,
    ocr_settings: OcrSettings,
    detection_settings: DetectionSettings,
    busy: bool,
    total_tasks: f32,
    completed_tasks: f32,
    progress: f32,
    sender: Sender<Command>,
    recv: Receiver<Command>,
    job_sender: Sender<Job>,
    job_recv: Receiver<Job>,
}

pub enum Command {
    AddScene {
        v_index: usize,
        scene: Scene,
    },
    RemoveScene {
        v_index: usize,
        s_index: usize,
    },
    AddVideo {
        video: Video,
    },
    RemoveVideo {
        v_index: usize,
    },
    UpdateThumbnail {
        v_index: usize,
        s_index: usize,
        image: Option<egui::TextureId>,
    },
    ExportOrg {
        filename: String,
    },
    ExportMarkdown {
        filename: String,
    },
    ExportPlayList {
        filename: String,
    },
    AddPendingTasks {
        tasks: usize,
    },
}

pub enum Job {
    DetectScenes {
        v_index: usize,
        file: String,
    },
    CreateThumbnail {
        v_index: usize,
        s_index: usize,
        imageFn: fn(
            frame: &epi::Frame,
            creator: String,
            title: String,
            s: &Scene,
        ) -> Option<egui::TextureId>,
    },
}

impl Default for App {
    fn default() -> Self {
        let (sender, recv) = channel();
        let (job_sender, job_recv) = channel();
        Self {
            icons: HashMap::new(),
            file: BLANK.to_owned(), //refers to the index file (save file)
            last_selected_file: BLANK.to_owned(), //refers to the index file (save file)
            instructional: Instructional {
                creator: BLANK.to_owned(),
                title: BLANK.to_owned(),
                url: BLANK.to_owned(),
                timestamps: BLANK.to_owned(),
                videos: vec![],
            },
            candidate_creators: vec![
                "John Danaher",
                "Gordon Ryan",
                "Craig Jones",
                "Lachlan Giles",
                "Mikey Musumeci",
                "Marcelo Garcia",
                "Bernando Faria",
                "Marcus Buchecha Almeida",
                "Andre Galvao",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            use_creator_combo: false,
            use_title_combo: false,
            candidate_titles: vec![],
            candidate_urls: vec![],
            scene_images: vec![],
            general_settings: GeneralSettings::new(),
            ocr_settings: OcrSettings::new(),
            detection_settings: DetectionSettings::new(),
            busy: true,
            total_tasks: 0.0,
            completed_tasks: 0.0,
            progress: 0.0,
            sender,
            recv,
            job_sender,
            job_recv,
        }
    }
}

impl epi::App for App {
    fn name(&self) -> &str {
        "eframe template"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
        self.icons.insert(
            "add-circle-line.png",
            load_texture_id(frame, get_icon("add-circle-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "add-line",
            load_texture_id(frame, get_icon("add-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "character-recognition-line",
            load_texture_id(frame, get_icon("character-recognition-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "close-line",
            load_texture_id(frame, get_icon("close-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "delete-bin-line",
            load_texture_id(frame, get_icon("delete-bin-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "download-cloud-line",
            load_texture_id(frame, get_icon("download-cloud-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "download-line",
            load_texture_id(frame, get_icon("download-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "drop-line",
            load_texture_id(frame, get_icon("drop-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "eject-line",
            load_texture_id(frame, get_icon("eject-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "film-line",
            load_texture_id(frame, get_icon("film-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "folder-download-line",
            load_texture_id(frame, get_icon("folder-download-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "hammer-line",
            load_texture_id(frame, get_icon("hammer-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "keyboard-box-line",
            load_texture_id(frame, get_icon("keyboard-box-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "menu-line",
            load_texture_id(frame, get_icon("menu-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "mouse-line",
            load_texture_id(frame, get_icon("mouse-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "play-line",
            load_texture_id(frame, get_icon("play-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "rewind-line",
            load_texture_id(frame, get_icon("rewind-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "rewind-mini-line",
            load_texture_id(frame, get_icon("rewind-mini-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "search-line",
            load_texture_id(frame, get_icon("search-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "speed-line",
            load_texture_id(frame, get_icon("speed-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "speed-mini-fill",
            load_texture_id(frame, get_icon("speed-mini-fill.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "split-cells-horizontal",
            load_texture_id(frame, get_icon("split-cells-horizontal.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "split-cells-vertical",
            load_texture_id(frame, get_icon("split-cells-vertical.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "star-line",
            load_texture_id(frame, get_icon("star-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "star-half-line",
            load_texture_id(frame, get_icon("star-half-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "toggle-line",
            load_texture_id(frame, get_icon("toggle-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "tools-line",
            load_texture_id(frame, get_icon("tools-line.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "arrow-up",
            load_texture_id(frame, get_icon("arrow-up-fill.png").as_path()).unwrap(),
        );
        self.icons.insert(
            "arrow-down",
            load_texture_id(frame, get_icon("arrow-down-fill.png").as_path()).unwrap(),
        );
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
        let Self {
            icons,
            file,
            last_selected_file,
            instructional,
            candidate_creators,
            use_creator_combo,
            candidate_titles,
            use_title_combo,
            candidate_urls,
            scene_images,
            general_settings,
            ocr_settings,
            detection_settings,
            busy,
            completed_tasks,
            total_tasks,
            progress,
            sender,
            recv,
            job_sender,
            job_recv,
        } = self;

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        *file = "".to_owned();
                        *instructional = Instructional {
                            creator: BLANK.to_owned(),
                            title: BLANK.to_owned(),
                            url: BLANK.to_owned(),
                            timestamps: BLANK.to_owned(),
                            videos: vec![],
                        };
                        *use_creator_combo = false;
                        *candidate_creators = get_cached_creators();
                        *use_title_combo = false;
                        *candidate_titles = vec![];
                        *candidate_urls = vec![];
                        *scene_images = vec![];
                    }
                    if ui.button("Open").clicked() {
                        let dir = parent_dir(&file).unwrap_or_else(|| {
                            parent_dir(last_selected_file).unwrap_or("/".to_string())
                        });
                        let target = rfd::FileDialog::new()
                            .add_filter("Org files", &["org"])
                            .set_directory(dir)
                            .pick_file()
                            .map(|f| {
                                f.as_path()
                                    .to_str()
                                    .expect("Failed to get path from dialog.")
                                    .to_string()
                            });

                        match target {
                            Some(f) => {
                                *last_selected_file = f.clone();
                                *file = f.clone();
                                *instructional = load_org(File::open(&f).unwrap());
                                *scene_images = allocate_scene_images(frame, &instructional.videos);
                                *total_tasks += instructional
                                    .videos
                                    .iter()
                                    .map(|v| v.scenes.len())
                                    .reduce(|a, b| a + b)
                                    .unwrap_or_default()
                                    as f32;
                                for i in 0..instructional.videos.len() {
                                    for j in 0..instructional.videos[i].scenes.len() {
                                        job_sender
                                            .send(Job::CreateThumbnail {
                                                v_index: i,
                                                s_index: j,
                                                imageFn: create_scene_image,
                                            })
                                            .expect("Failed to send CreateThumbnail command!");
                                    }
                                }
                            }
                            None => {}
                        };
                    };
                    if !&file.is_empty() && ui.button("Save").clicked() {
                        match File::create(&file) {
                            Ok(f) => save_org(instructional, f, true),
                            Err(_) => {}
                        };
                    }
                    if ui.button("Save as").clicked() {
                        let dir = parent_dir(&file).unwrap_or_else(|| {
                            parent_dir(last_selected_file).unwrap_or("/".to_string())
                        });
                        let target = rfd::FileDialog::new()
                            .add_filter("Org files", &["org"])
                            .set_directory(dir)
                            .save_file()
                            .map(|f| {
                                f.as_path()
                                    .to_str()
                                    .expect("Failed to get path from dialog.")
                                    .to_string()
                            });

                        match target {
                            Some(t) => {
                                *last_selected_file = t.clone();
                                let target_path = Path::new(&t);
                                let target_file = File::create(target_path)
                                    .expect("Failed to open file for saving!");
                                save_org(instructional, target_file, true);
                            }
                            None => {}
                        };
                    }

                    if ui.button("Export as markdown").clicked() {
                        let dir = parent_dir(&file).unwrap_or_else(|| {
                            parent_dir(last_selected_file).unwrap_or("/".to_string())
                        });
                        let target = rfd::FileDialog::new()
                            .add_filter("Markdown files", &["md"])
                            .set_directory(dir)
                            .save_file()
                            .map(|f| {
                                f.as_path()
                                    .to_str()
                                    .expect("Failed to get path from dialog.")
                                    .to_string()
                            });

                        match target {
                            Some(t) => {
                                *last_selected_file = t.clone();
                                let target_path = Path::new(&t);
                                let target_file = File::create(target_path)
                                    .expect("Failed to open file for saving!");
                                save_md(instructional, target_file);
                            }
                            None => {}
                        };
                    }

                    if ui.button("Export as playlist").clicked() {
                        let dir = parent_dir(&file).unwrap_or_else(|| {
                            parent_dir(last_selected_file).unwrap_or("/".to_string())
                        });
                        let target = rfd::FileDialog::new()
                            .add_filter("Playlist files", &["m3u"])
                            .set_directory(dir)
                            .save_file()
                            .map(|f| {
                                f.as_path()
                                    .to_str()
                                    .expect("Failed to get path from dialog.")
                                    .to_string()
                            });

                        match target {
                            Some(t) => {
                                *last_selected_file = t.clone();
                                let target_path = Path::new(&t);
                                let target_file = File::create(target_path)
                                    .expect("Failed to open file for saving!");
                                save_playlist(instructional, target_file);
                            }
                            None => {}
                        };
                    }

                    if ui.button("Split").clicked() {
                        let old = instructional.clone();
                        let target = instructional.clone();
                        let sender = sender.clone();

                        let org_export_enabled = general_settings.org_export_enabled.clone();
                        let md_export_enabled = general_settings.md_export_enabled.clone();
                        let playlist_export_enabled = general_settings.playlist_export_enabled.clone();

                        let org_export_filename = general_settings.org_export_filename.clone();
                        let md_export_filename = general_settings.md_export_filename.clone();
                        let playlist_export_filename = general_settings.playlist_export_filename.clone();

                        instructional.videos = vec![];
                        *scene_images = Vec::new();
                        std::thread::spawn(move || {
                            let all_scenes = all_scenes(old);
                            sender
                                .send(Command::AddPendingTasks {
                                    tasks: all_scenes.len(),
                                })
                                .expect("Failed to send AddPendingTasks command!");
                            all_scenes.iter().enumerate().for_each(|(i, s)| {
                                if let Some(v) = split_scene(i + 1, s.clone()) {
                                    sender.send(Command::AddVideo { video: v })
                                        .expect("Failed to send AddVideo command!");
                                }
                            });
                            if org_export_enabled {
                                sender.send(Command::ExportOrg  { filename: org_export_filename.to_string()  });
                            }
                            if md_export_enabled {
                                sender.send(Command::ExportMarkdown  { filename: md_export_filename.to_string() });
                            }
                            if playlist_export_enabled {
                                sender.send(Command::ExportPlayList { filename: playlist_export_filename.to_string() });
                            }
                        });
                    }

                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
                ui.menu_button("Tools", |ui| {
                    if ui.button("Update cache").clicked() {
                        update_cache("".to_string(), "".to_string());
                    }
                });
                ui.menu_button("Import", |ui| {
                    if ui.button("Video").clicked() {
                        add_video(&mut instructional.videos, last_selected_file);
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        //TODO: implement about
                    }
                });
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.heading("Hackers guide");
                ui.heading("to");
                ui.heading("Jiu Jitsu");
                ui.hyperlink("https://github.com/iocanel/hg2jj");
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    let progress_bar = egui::ProgressBar::new(*progress as f32)
                        .show_percentage()
                        .animate(true);

                    if *total_tasks > 0.0 {
                        *busy = ui.add(progress_bar).hovered();
                    }

                    //reset the progress
                    if *completed_tasks >= *total_tasks {
                        *completed_tasks = 0.0;
                        *total_tasks = 0.0;
                        *progress = 0.0
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.label("Creator: ");
            ui.horizontal(|ui| {
                if *use_creator_combo {
                    if instructional.creator.is_empty() {
                        instructional.creator = candidate_creators[0].to_string();
                    }
                    if ui.add(egui::ImageButton::new(*icons.get("keyboard-box-line").unwrap(), (10.0, 10.0))).on_hover_text("Enter creator manually").clicked() {
                        *use_creator_combo=false;
                    }

                    let popular = get_popular_creators();
                    if candidate_creators.len() == popular.len() {

                        if ui.add(egui::ImageButton::new(*icons.get("star-line").unwrap(), (10.0, 10.0))).on_hover_text("Show all").clicked() {
                            *candidate_creators = get_cached_creators();
                        }
                    } else {
                        if ui.add(egui::ImageButton::new(*icons.get("star-half-line").unwrap(), (10.0, 10.0))).on_hover_text("Only show popular").clicked() {
                            *candidate_creators = popular;
                        }
                    }
                    egui::ComboBox::from_label("Select creator")
                        .selected_text(format!("{:?}", instructional.creator))
                        .show_ui(ui, |ui| {
                            candidate_creators.iter()
                                .for_each(|t| {
                                    if ui.selectable_value(&mut instructional.creator, t.to_string(), t).changed() {
                                        refresh_titles(instructional, candidate_urls, candidate_titles);
                                    }
                                });
                        });
                } else {
                    if ui.add(egui::ImageButton::new(*icons.get("mouse-line").unwrap(), (10.0, 10.0))).on_hover_text("Select creator from combobox").clicked() {
                        *use_creator_combo=true;
                    }
                    ui.text_edit_singleline(&mut instructional.creator);
                }
            });
            ui.label("Title: ");
            ui.horizontal(|ui| {
                if *use_title_combo {

                    if ui.add(egui::ImageButton::new(*icons.get("keyboard-box-line").unwrap(), (10.0, 10.0))).on_hover_text("Enter title manually").clicked() {
                        *use_title_combo=false;
                    }
                    if !candidate_titles.is_empty() {
                        if instructional.title.is_empty() {
                            instructional.title = candidate_titles[0].to_string();
                        }
                        egui::ComboBox::from_label("Select title")
                            .selected_text(format!("{:?}", instructional.title))
                            .show_ui(ui, |ui| {
                                candidate_titles.iter().for_each(|t| {
                                    ui.selectable_value(&mut instructional.title, t.to_string(), t);
                                });
                            });
                    }
                } else {

                    if ui.add(egui::ImageButton::new(*icons.get("mouse-line").unwrap(), (10.0, 10.0))).on_hover_text("Select title from a combobox").clicked() {
                        *use_title_combo=true;
                        refresh_titles(instructional, candidate_urls, candidate_titles);
                    }
                    ui.text_edit_singleline(&mut instructional.title);
                }
            });

            ui.label("Instructional URL: ");
            ui.horizontal(|ui| {
                let index = candidate_titles.iter().position(|t| t.eq(&instructional.title)).unwrap_or_default();
                if candidate_urls.len() > index {
                    instructional.url = candidate_urls[index].to_string();
                }

                ui.add_sized(Vec2::new(ui.available_size().x - 100.0, ui.available_size().y) , egui::TextEdit::singleline(&mut instructional.url));
                if ui.add(egui::ImageButton::new(*icons.get("download-cloud-line").unwrap(), (10.0, 10.0))).on_hover_text("Download timestamps").clicked() {
                    //Scrap instuctional info but try to retain things like associated files, labels etc
                    instructional.timestamps = scrape_url(instructional.url.to_string());
                } 

                if !instructional.timestamps.is_empty() && ui.add(egui::ImageButton::new(*icons.get("arrow-down").unwrap(), (10.0, 10.0))).on_hover_text("Apply timestamps").clicked() {
                    instructional.videos = extract_timestamps(instructional.timestamps.clone())
                        .iter()
                        .filter(|s| !s.is_empty())
                        .enumerate()
                        .map(|(i, s)| (i, s, if instructional.videos.len() > i { instructional.videos[i].file.clone() } else { format!("Volume{}.mp4", i + 1) }))
                        .map(|(i, s, file) | Video {index: i + 1, file: file.clone(), scenes: s.iter().map(|s| Scene { index: s.index, title: s.title.clone(), start: s.start, end: s.end, file: file.clone(), labels: s.labels.to_vec()}).collect(), duration: 0})
                        .collect();
                }
            });

            if !instructional.timestamps.is_empty() {
                egui::CollapsingHeader::new("Scraped text").id_source(Id::new("scraped-text")).default_open(false).show(ui, |ui| { 
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.add_sized(Vec2::new(ui.available_size().x - 100.0, ui.available_size().y - 300.0) , egui::TextEdit::multiline(&mut instructional.timestamps));
                        });
                });
            }

            ui.horizontal(|ui| {
                ui.heading("Videos");
                if ui.add(egui::ImageButton::new(*icons.get("add-line").unwrap(), (10.0, 10.0))).on_hover_text("Add video").clicked() {
                    add_video(&mut instructional.videos, last_selected_file);
                }
            });

            ui.separator();

            egui::CollapsingHeader::new("Settings").id_source(Id::new("settings")).default_open(false).show(ui, |ui| { 
                egui::CollapsingHeader::new("General Settings").id_source(Id::new("general")).default_open(true).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Step in seconds");
                        ui.add(egui::Slider::new(&mut general_settings.step_in_secs, 1..=10)).on_hover_text("The number of seconds to use when offsetting scenes of this video.");
                        if ui.add(egui::ImageButton::new(*icons.get("rewind-line").unwrap(), (10.0, 10.0))).on_hover_text(format!("Subtract {} second(s) from all scenes", general_settings.step_in_secs)).clicked() {

                            for i in 0..instructional.videos.len() {
                                for j in 0..instructional.videos[i].scenes.len() {
                                    instructional.videos[i].scenes[j].start-=general_settings.step_in_secs.to_owned();
                                }
                                for j in 0..instructional.videos[i].scenes.len() {
                                    sync_scene_start(&mut instructional.videos[i], j);
                                    //We need to clone things that we pass to the thread.
                                    sender.send(Command::AddPendingTasks{tasks: 1});
                                    job_sender.send(Job::CreateThumbnail{ v_index: i, s_index: j, imageFn: create_scene_image}).expect("Failed to send CreateThumbnail command!");
                                }
                            }
                        }
                        if ui.add(egui::ImageButton::new(*icons.get("speed-line").unwrap(), (10.0, 10.0))).on_hover_text(format!("Add {} second(s) to all scenes", general_settings.step_in_secs)).clicked() {

                            for i in 0..instructional.videos.len() {
                                for j in 0..instructional.videos[i].scenes.len() {
                                    instructional.videos[i].scenes[j].start+=general_settings.step_in_secs.to_owned();
                                }
                                for j in 0..instructional.videos[i].scenes.len() {
                                    sync_scene_start(&mut instructional.videos[i], j);
                                    //We need to clone things that we pass to the thread.
                                    let sender = sender.clone();
                                    sender.send(Command::AddPendingTasks{tasks: 1});
                                    job_sender.send(Job::CreateThumbnail{ v_index: i, s_index: j, imageFn: create_scene_image}).expect("Failed to send CreateThumbnail command!");
                                }
                            }
                        }
                    });
                });

                egui::CollapsingHeader::new("Export Settings").id_source(Id::new("export")).default_open(false).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Export org file name:");
                        ui.add(egui::Checkbox::new(&mut general_settings.org_export_enabled, "Export org enabled"));
                        ui.add_sized(Vec2::new(100.0, ui.available_size().y) , egui::TextEdit::singleline(&mut general_settings.org_export_filename));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Export markdown file name:");
                        ui.add(egui::Checkbox::new(&mut general_settings.playlist_export_enabled, "Export markdwon enabled"));
                        ui.add_sized(Vec2::new(100.0, ui.available_size().y) , egui::TextEdit::singleline(&mut general_settings.md_export_filename));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Export playlist name:");
                        ui.add(egui::Checkbox::new(&mut general_settings.playlist_export_enabled, "Export playlist enabled"));
                        ui.add_sized(Vec2::new(100.0, ui.available_size().y) , egui::TextEdit::singleline(&mut general_settings.playlist_export_filename));
                    });
                });
                egui::CollapsingHeader::new("Detection Settings").id_source(Id::new("detection")).default_open(false).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Slider::new(&mut detection_settings.threshold, 0.0..=1.0).text("Scene detection threshold")).on_hover_text("What percentage of changes pixels suggestes a scene change?");
                        ui.add(egui::Slider::new(&mut detection_settings.minimum_length, 0..=3600).text("Minimum scene length")).on_hover_text("What is the minimum expected scene length? Shorter scenes will be ignored!");
                    });
                });

                egui::CollapsingHeader::new("OCR Settings").id_source(Id::new("ocr")).default_open(false).show(ui, |ui| {
                    ui.add(egui::Checkbox::new(&mut ocr_settings.grayscale, "Grayscle"));
                    ui.horizontal(|ui| {
                        ui.add(egui::Checkbox::new(&mut ocr_settings.gaussian_thresholding, "Gaussian Thresholding"));
                        ui.add(egui::Slider::new(&mut ocr_settings.gaussian_thresholding_max_value, 0.0..=255.0));
                        if ui.add(egui::Slider::new(&mut ocr_settings.gaussian_thresholding_blocksize, 1..=200)).changed() {
                            if ocr_settings.gaussian_thresholding_blocksize % 2 == 0 {
                                ocr_settings.gaussian_thresholding_blocksize += 1;
                            }
                        }
                        ui.add(egui::Slider::new(&mut ocr_settings.gaussian_thresholding_c, -30.0..=30.0));
                    });

                    ui.horizontal(|ui| {
                        ui.add(egui::Checkbox::new(&mut ocr_settings.otsu_thresholding, "Otsu Thresholding"));
                        ui.add(egui::Slider::new(&mut ocr_settings.otsu_thresholding_min_value, 0.0..=255.0));
                        ui.add(egui::Slider::new(&mut ocr_settings.otsu_thresholding_max_value, 0.0..=255.0));
                    });
                    ui.add(egui::Checkbox::new(&mut ocr_settings.invert, "Invert"));


                    ui.horizontal(|ui| {
                        ui.add(egui::Checkbox::new(&mut ocr_settings.denoise, "Denoise"));
                        ui.add(egui::Slider::new(&mut ocr_settings.denoise_strength, 1.0..=100.0)).on_hover_text("Strength");
                    });
                    ui.horizontal(|ui| {
                        ui.add(egui::Checkbox::new(&mut ocr_settings.erode, "Erode"));
                        ui.add(egui::Slider::new(&mut ocr_settings.erode_kernel_size, 1..=10)).on_hover_text("Kernel size");
                        ui.add(egui::Slider::new(&mut ocr_settings.erode_iterations, 1..=5)).on_hover_text("Iterations");
                    });
                    ui.horizontal(|ui| {
                        ui.add(egui::Checkbox::new(&mut ocr_settings.dilate, "Dilate"));
                        ui.add(egui::Slider::new(&mut ocr_settings.dilate_kernel_size, 1..=10)).on_hover_text("Kernel size");
                        ui.add(egui::Slider::new(&mut ocr_settings.dilate_iterations, 1..=5)).on_hover_text("Iterations");
                    });
                    ui.add(egui::Checkbox::new(&mut ocr_settings.spellcheking, "Spell check"));
                    egui::ComboBox::from_label( "Character case").selected_text(format!("{:?}", ocr_settings.case)).show_ui(ui, |ui| {
                        ui.selectable_value(&mut ocr_settings.case, crate::Case::CapitalizeFirst, "Capitalize First");
                        ui.selectable_value(&mut ocr_settings.case, crate::Case::Upper, "Upper");
                        ui.selectable_value(&mut ocr_settings.case, crate::Case::Lower, "Lower");
                    });
                });
            });

            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {

                    let mut source_scene_title = None;
                    let mut drop_scene = None;
                    for i in 0..instructional.videos.len() {
                        egui::CollapsingHeader::new(format!("{}", i + 1)).id_source(Id::new("video").with(i)).default_open(true).show(ui, |ui| { 
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.text_edit_singleline(&mut instructional.videos[i].file);

                                        if ui.add(egui::ImageButton::new(*icons.get("film-line").unwrap(), (10.0, 10.0))).on_hover_text("Select video").clicked() {
                                            let dir = video_dir(&instructional.videos[i])
                                                .unwrap_or_else(||
                                                                scenes_dir(&instructional.videos[i].scenes)
                                                                .unwrap_or_else(||
                                                                                parent_dir(&last_selected_file.clone()).unwrap_or("/".to_string())));
                                            let file = rfd::FileDialog::new()
                                                .add_filter("Video files", &["avi", "mpg", "mp4", "mkv"])
                                                .set_directory(dir)
                                                .pick_file();

                                            match file {
                                                Some(f) => {
                                                    let f_str = f.as_path().to_str().unwrap().to_string();
                                                    *last_selected_file = f_str.clone();
                                                    for s in 0..instructional.videos[i].scenes.len() {
                                                        instructional.videos[i].scenes[s].file = f_str.clone();
                                                    }
                                                    instructional.videos[i].file = f_str.clone();
                                                    let duration = video_duration(f_str);
                                                    instructional.videos[i].duration = duration;
                                                    if instructional.videos[i].scenes.len() > 0 {
                                                        let last_index: usize = instructional.videos[i].scenes.len() - 1;
                                                        instructional.videos[i].scenes[last_index].end = duration;
                                                    }
                                                    *scene_images = reallocate_scene_images(frame, &instructional.videos, i, scene_images.clone());
                                                    let pending = sender.clone();
                                                    let tasks = instructional.videos[i].scenes.len();
                                                    pending.send(Command::AddPendingTasks{tasks}).expect("Failed to send AddPendingTasks command!");
                                                    //We need to clone things that we pass to the thread.
                                                    for si in 0..instructional.videos[i].scenes.len() {
                                                        job_sender.send(Job::CreateThumbnail{ v_index: i, s_index: si, imageFn: create_scene_image}).expect("Failed to send CreateThumbnail commnad!");
                                                    }
                                                },
                                                None =>  {}
                                            };
                                        }

                                        if ui.add(egui::ImageButton::new(*icons.get("eject-line").unwrap(), (10.0, 10.0))).on_hover_text("Eject video").clicked() {
                                            sender.send(Command::RemoveVideo{ v_index: i }).expect("Failed to send Remove Video Command");
                                        }

                                        if ui.add(egui::ImageButton::new(*icons.get("search-line").unwrap(), (10.0, 10.0))).on_hover_text("Detect scenes").clicked() {
                                            let file = instructional.videos[i].file.clone();
                                            let sender = sender.clone();
                                            job_sender.send(Job::DetectScenes{v_index: i, file: Path::new(&file).as_os_str().to_str().unwrap().to_string()}).expect("Failed to send Detect scenes command");
                                        }

                                        if ui.add(egui::ImageButton::new(*icons.get("add-line").unwrap(), (10.0, 10.0))).on_hover_text("Add scene").clicked() {
                                            let s_index = instructional.videos[i].scenes.len() + 1;
                                            let previous_end = if s_index > 1 { instructional.videos[i].scenes[s_index - 1].end } else { 0 };
                                            let end = video_duration(instructional.videos[i].file.to_string());
                                            let scene = Scene { index: s_index, title: "".to_string(), file: instructional.videos[i].file.clone(), labels: vec![], start: previous_end, end: end };
                                            sender.send(Command::AddScene{v_index: i, scene: scene.clone() }).expect("Failed to send AddScene command");
                                            sender.send(Command::UpdateThumbnail{v_index: i, s_index, image: create_scene_image(&frame, instructional.creator.to_string(), instructional.title.to_string(), &scene)}).expect("Failed to send UpdateThumbnail command!");
                                        }
                                    });

                                    // Global video actions
                                    ui.horizontal(|ui| {
                                        if ui.add(egui::ImageButton::new(*icons.get("rewind-line").unwrap(), (10.0, 10.0))).on_hover_text(format!("Subtract {} second(s) from all scenes", general_settings.step_in_secs)).clicked() {
                                            for j in 0..instructional.videos[i].scenes.len() {
                                                instructional.videos[i].scenes[j].start-=general_settings.step_in_secs.to_owned();
                                            }
                                            for j in 0..instructional.videos[i].scenes.len() {
                                                sync_scene_start(&mut instructional.videos[i], j);
                                                //We need to clone things that we pass to the thread.
                                                sender.send(Command::AddPendingTasks{tasks: 1});
                                                job_sender.send(Job::CreateThumbnail{ v_index: i, s_index: j, imageFn: create_scene_image}).expect("Failed to send CreateThumbnail command!");
                                            }
                                        }
                                        if ui.add(egui::ImageButton::new(*icons.get("speed-line").unwrap(), (10.0, 10.0))).on_hover_text(format!("Add {} second(s) to all scenes", general_settings.step_in_secs)).clicked() {
                                            for j in 0..instructional.videos[i].scenes.len() {
                                                instructional.videos[i].scenes[j].start+=general_settings.step_in_secs.to_owned();
                                            }
                                            for j in 0..instructional.videos[i].scenes.len() {
                                                sync_scene_start(&mut instructional.videos[i], j);
                                                //We need to clone things that we pass to the thread.
                                                let sender = sender.clone();
                                                sender.send(Command::AddPendingTasks{tasks: 1});
                                                job_sender.send(Job::CreateThumbnail{ v_index: i, s_index: j, imageFn: create_scene_image}).expect("Failed to send CreateThumbnail command!");
                                            }
                                        }
                                        if ui.add(egui::ImageButton::new(*icons.get("arrow-up").unwrap(), (10.0, 10.0))).on_hover_text("Shift titles up").clicked() {
                                            for j in 1..instructional.videos[i].scenes.len() {
                                                instructional.videos[i].scenes[j - 1].title=instructional.videos[i].scenes[j].title.to_string();
                                            }
                                        }
                                        if ui.add(egui::ImageButton::new(*icons.get("arrow-down").unwrap(), (10.0, 10.0))).on_hover_text("Shift titles down").clicked() {
                                            for j in 1..instructional.videos[i].scenes.len() {
                                                let k = instructional.videos[i].scenes.len() - j;
                                                println!("Title: {} = {}.", k, k-1);
                                                instructional.videos[i].scenes[k].title=instructional.videos[i].scenes[k - 1].title.to_string();
                                            }
                                        }
                                    });

                                    // Scenes - start
                                    for j in 0..instructional.videos[i].scenes.len() {
                                        let drop = drop_target(ui, true, |ui| {
                                            ui.horizontal(|ui| {
                                            let scene_title_id = Id::new("scene_title").with(i).with(j);
                                            if ui.add(egui::ImageButton::new(*icons.get("split-cells-vertical").unwrap(), (10.0, 10.0))).on_hover_text("Split scene").clicked() {
                                                let scene_to_add = instructional.videos[i].scenes[j].clone();
                                                instructional.videos[i].scenes.insert(j, scene_to_add);
                                                if scene_images.len() > i && scene_images[i].len() > j {
                                                    let image_to_add = scene_images[i][j].clone();
                                                    scene_images[i].insert(j, image_to_add);
                                                }
                                            }
 
                                            ui.separator();
                                            drag_source(ui, scene_title_id, |ui| {
                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                       ui.add_sized(Vec2::new(ui.available_size().x - 100.0, ui.available_size().y) , egui::TextEdit::singleline(&mut instructional.videos[i].scenes[j].title));
                                                    });
                                                });
                                            });
                                            ui.separator();

                                            if ui.add(egui::ImageButton::new(*icons.get("close-line").unwrap(), (10.0, 10.0))).on_hover_text("Remove scene").clicked() {
                                                sender.send(Command::RemoveScene{ v_index: i, s_index: j }).expect("Failed to send Remove Scene Command");
                                            }

                                            if ui.memory().is_being_dragged(scene_title_id) {
                                                source_scene_title = Some((i, j));
                                            }
                                            
                                        });
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                let lower_bound = 0;
                                                let higher_bound = 3*60*60;
 
                                                // let lower_bound = if j == 0 { 0 } else { instructional.videos[i].scenes[j - 1].end };
                                                // let higher_bound = if j + 1 == instructional.videos[i].scenes.len() { instructional.videos[i].duration } else { instructional.videos[i].scenes[j + 1].start };
                                                ui.label(format!("Start: {}", seconds_to_time(instructional.videos[i].scenes[j].start)));
                                                ui.horizontal(|ui| {
                                                    let mut refresh_needed = false;
                                                    if ui.add(egui::ImageButton::new(*icons.get("rewind-line").unwrap(), (10.0, 10.0))).on_hover_text(format!("Subtract {} second(s)", general_settings.step_in_secs)).clicked() {
                                                        instructional.videos[i].scenes[j].start-=general_settings.step_in_secs.to_owned();
                                                        refresh_needed = true;
                                                    }
                                                    if ui.add(egui::ImageButton::new(*icons.get("speed-line").unwrap(), (10.0, 10.0))).on_hover_text(format!("Add {} second(s)", general_settings.step_in_secs)).clicked() {
                                                        instructional.videos[i].scenes[j].start+=general_settings.step_in_secs.to_owned();
                                                        sync_scene_start(&mut instructional.videos[i], j);
                                                        refresh_needed = true;
                                                    }
                                                    if instructional.videos[i].scenes[j].start > instructional.videos[i].scenes[j].end && instructional.videos[i].scenes[j].end > 0 &&
                                                        ui.add(egui::ImageButton::new(*icons.get("tools-line").unwrap(), (10.0, 10.0)))
                                                        .on_hover_text(format!("Recommended fix:{}", instructional.videos[i].scenes[j].start/60)).clicked() {
                                                        instructional.videos[i].scenes[j].start/=60;
                                                        sync_scene_start(&mut instructional.videos[i], j);
                                                        refresh_needed = true;
                                                    }

                                                    if refresh_needed {
                                                       //We need to clone things that we pass to the thread.
                                                       sender.send(Command::AddPendingTasks{tasks: 1});
                                                       job_sender.send(Job::CreateThumbnail{ v_index: i, s_index: j, imageFn: create_scene_image}).expect("Failed to send CreateThumbnail command!");
                                                    }
                                                });
                                                if ui.add(egui::Slider::new(&mut instructional.videos[i].scenes[j].start, lower_bound..=higher_bound)).changed() {
                                                    sync_scene_start(&mut instructional.videos[i], j);
                                                }
                                                ui.label(format!("End: {}", seconds_to_time(instructional.videos[i].scenes[j].end)));
                                                ui.horizontal(|ui| {
                                                   let mut refresh_needed = false;
                                                    if ui.add(egui::ImageButton::new(*icons.get("rewind-line").unwrap(), (10.0, 10.0))).on_hover_text(format!("Subtract {} second(s)", general_settings.step_in_secs)).clicked() {
                                                        instructional.videos[i].scenes[j].end-=general_settings.step_in_secs.to_owned();
                                                        sync_scene_end(&mut instructional.videos[i], j);
                                                        refresh_needed = true;
                                                    }
                                                    if ui.add(egui::ImageButton::new(*icons.get("speed-line").unwrap(), (10.0, 10.0))).on_hover_text(format!("Add {} second(s)", general_settings.step_in_secs)).clicked() {
                                                        instructional.videos[i].scenes[j].end+=general_settings.step_in_secs.to_owned();
                                                        sync_scene_end(&mut instructional.videos[i], j);
                                                        refresh_needed = true;
                                                    }

                                                    if j + 1 < instructional.videos[i].scenes.len()
                                                        && instructional.videos[i].scenes[j].end > instructional.videos[i].scenes[j + 1].end 
                                                        && ui.add(egui::ImageButton::new(*icons.get("tools-line").unwrap(), (10.0, 10.0))).on_hover_text(format!("Recommended fix: {}", instructional.videos[i].scenes[j].end/60)).clicked() {
                                                            instructional.videos[i].scenes[j].end/=60;
                                                            instructional.videos[i].scenes[j + 1].start=instructional.videos[i].scenes[j].end;
                                                            refresh_needed = true;
                                                        } else if j + 1 == instructional.videos[i].scenes.len() && instructional.videos[i].scenes[j].end == 0
                                                        && ui.add(egui::ImageButton::new(*icons.get("tools-line").unwrap(), (10.0, 10.0))).on_hover_text(format!("Recommended fix: video duration")).clicked() {
                                                            instructional.videos[i].scenes[j].end=video_duration(instructional.videos[i].file.to_string());
                                                            refresh_needed = true;
                                                        }
                                                    if refresh_needed {
                                                       //We need to clone things that we pass to the thread.
                                                       sender.send(Command::AddPendingTasks{tasks: 1});
                                                       job_sender.send(Job::CreateThumbnail{ v_index: i, s_index: j, imageFn: create_scene_image}).expect("Failed to send CreateThumbnail command!");
                                                    }
                                                });
                                                if ui.add(egui::Slider::new(&mut instructional.videos[i].scenes[j].end, lower_bound..=higher_bound)).changed() {
                                                    sync_scene_end(&mut instructional.videos[i], j);
                                                }
                                            });

                                            ui.separator();
                                            if scene_images.len() == i {
                                                scene_images.insert(i, vec![]);
                                            }
                                            if scene_images.len() > i && scene_images[i].len() == j {
                                                scene_images[i].insert(j, None);
                                            }

                                            if scene_images.len() > i && scene_images[i].len() > j {
                                                let mut size = egui::Vec2::new(192.0,102.0);
                                                size *= (ui.available_width() / size.x).min(1.0);
                                                match scene_images[i][j] {
                                                    Some(img) => {
                                                        if ui.add(egui::ImageButton::new(img, size)).clicked() {
                                                            //We need to clone things that we pass to the thread.
                                                            sender.send(Command::AddPendingTasks{tasks: 1});
                                                            job_sender.send(Job::CreateThumbnail{ v_index: i, s_index: j, imageFn: create_scene_image}).expect("Failed to send CreateThumbnail command!");
                                                        } 
                                                    },
                                                    None => {
                                                        if ui.add(egui::ImageButton::new(*icons.get("search-line").unwrap(), size)).clicked() {
                                                            //We need to clone things that we pass to the thread.
                                                            sender.send(Command::AddPendingTasks{tasks: 1});
                                                            job_sender.send(Job::CreateThumbnail{ v_index: i, s_index: j, imageFn: create_scene_image}).expect("Failed to send CreateThumbnail command!");
                                                        }

                                                    },
                                                }
                                                ui.separator();
                                                ui.vertical(|ui| {
                                                    if ui.add(egui::ImageButton::new(*icons.get("play-line").unwrap(), (10.0, 10.0))).on_hover_text("Play Video").clicked() {
                                                        let scene = instructional.videos[i].scenes[j].clone();
                                                        std::thread::spawn(move || {
                                                            play_scene(scene);
                                                        });
                                                    }
                                                    if ui.add(egui::ImageButton::new(*icons.get("character-recognition-line").unwrap(), (10.0, 10.0))).on_hover_text("Detect scene title using OCR").clicked() {
                                                        let scene = &instructional.videos[i].scenes[j];
                                                        if let Some(text) = scene_text_with_settings(instructional.creator.to_string(), instructional.title.to_string(), scene, &ocr_settings) {
                                                            job_sender.send(Job::CreateThumbnail{ v_index: i, s_index: j, imageFn: create_ocr_image}).expect("Failed to send CreateThumbnail command!");
                                                            instructional.videos[i].scenes[j].title =  text;
                                                        }
                                                    }
                                                });
                                            }
                                        });
                                        //end of drop target
                                        ui.separator();
                                    });                                    
                                        
                                    let is_being_dragged = ui.memory().is_anything_being_dragged();
                                    if is_being_dragged && drop.response.hovered() {
                                        drop_scene = Some((i, j));
                                    }
                                }
                                //after j loop
                            });
                        });
                        });
                    }

                    //Handle drag an drop
                    if let Some((s_video, s_scene)) = source_scene_title {
                        if let Some((d_video, d_scene)) = drop_scene {
                            if ui.input().pointer.any_released() {
                                // do the swap:
                                let source_title = instructional.videos[s_video].scenes[s_scene].title.clone();
                                instructional.videos[s_video].scenes[s_scene].title = instructional.videos[d_video].scenes[d_scene].title.clone();
                                instructional.videos[d_video].scenes[d_scene].title = source_title;
                            }
                        }
                    }

                    //Handle async UI actions
                    let mut commands:Vec<Command> = vec![];
                    let mut jobs:Vec<Job> = vec![];

                    while let Ok(command) = recv.try_recv() {
                        commands.push(command);
                    }

                    while let Ok(job) = job_recv.try_recv() {
                        jobs.push(job);
                    }

                    if !commands.is_empty() {
                        commands.into_iter().for_each(|command| {
                            match command {
                                Command::AddScene {v_index, scene} => {
                                    instructional.videos[v_index].scenes.push(scene.clone());
                                    let index = instructional.videos[v_index].scenes.len() - 1;
                                    update_scene_images(&frame, job_sender, v_index, index, instructional);
                                },
                                Command::RemoveScene {v_index, s_index} => {
                                    instructional.videos[v_index].scenes.remove(s_index);
                                    scene_images[v_index].remove(s_index);
                                },
                                Command::AddVideo {video} => {
                                    instructional.videos.push(video.clone());
                                    let index = instructional.videos.len() - 1;
                                    update_video_images(&frame, job_sender, index, instructional);
                                },
                                Command::RemoveVideo {v_index} => { instructional.videos.remove(v_index); },
                                Command::UpdateThumbnail {v_index, s_index, image } => {
                                    if scene_images.len() > v_index && scene_images[v_index].len() > s_index {
                                        scene_images[v_index][s_index] = image;
                                    }
                                    *completed_tasks += 1.0;
                                    *progress =  *completed_tasks / *total_tasks;
                                    println!("Completed: {} of {}.", completed_tasks, total_tasks);
                                },
                                Command::ExportPlayList {filename} => {
                                    save_playlist_in_dir(instructional, filename);
                                },
                                Command::ExportOrg {filename} => {
                                    save_org_in_dir(instructional, filename);
                                },
                                Command::ExportMarkdown {filename} => {
                                    save_md_in_dir(instructional, filename);
                                }
                                Command::AddPendingTasks {tasks} => {
                                    *total_tasks += tasks as f32;
                                }
                            } 
                        });
                    }

                    if !jobs.is_empty() {
                        let job_chunk_size = if jobs.len() < 3 { 1 } else { jobs.len() / 3 };
                        jobs.into_iter().chunks(job_chunk_size).into_iter().for_each(|chunk| {
                            let frame = frame.clone();
                            let instructional = instructional.clone();
                            let sender = sender.clone();
                            let detection_settings = detection_settings.clone();
                            let chunk_jobs = chunk.collect::<Vec<Job>>();
                            std::thread::spawn(move || {
                                chunk_jobs.into_iter().for_each(|job| {
                                    match job {
                                        Job::DetectScenes {v_index, file } => {
                                            println!("Detecting scenes for: {}", file.clone());
                                            let timestamps_with_scores: Vec<(usize, f32)> = scene_detect(file.clone(), detection_settings);
                                            let timestamps = timestamps_with_scores.into_iter()
                                                .map(|(t, s)| t)
                                                .fold(vec![0], |mut v, i| {v.push(i); v});

                                            println!("Detected {} scenes.", timestamps.len());
                                            timestamps.iter().for_each(|t| println!("\tTimestamp: {}", t));

                                            timestamps.to_vec() //t
                                                .into_iter()
                                                .zip_longest(timestamps.into_iter().skip(1)) //( t, nt)
                                                .map(|pair| {
                                                    match pair {
                                                        Both(l, r) => (l, r),
                                                        Right(r) => ((0 as usize), r),
                                                        Left(l) => (l, (0 as usize)),
                                                    }
                                                })
                                                .filter(|(t, n)| (*t as i32 - *n as i32).abs() > detection_settings.minimum_length) // filter very short scenes
                                                .enumerate() // (i, (t, nt))
                                                .map(| (si, (t, nt)) | (si, Scene {index: si, title: format!("Scene {}: {} - {}", si+1, t, nt), labels: vec![], file: file.clone(), start: t, end: nt}))
                                                .for_each(|(s_index, scene)| {
                                                    sender.send(Command::AddScene{v_index, scene: scene.to_owned()}).expect("Failed to send AddScene command");
                                                    sender.send(Command::UpdateThumbnail{v_index, s_index, image: create_scene_image(&frame, instructional.creator.to_string(), instructional.title.to_string(), &scene)}).expect("Failed to send UpdateThumbnail command!");
                                                });
                                        },
                                        Job::CreateThumbnail {v_index, s_index, imageFn } => {
                                            let scene = instructional.videos[v_index].scenes[s_index].clone();
                                            sender.send(Command::UpdateThumbnail{v_index, s_index, image: imageFn(&frame, instructional.creator.to_string(), instructional.title.to_string(), &scene)}).expect("Failed to send UpdateThumbnail command!");
                                        },
                                    }
                                });
                            });
                        });
                    }
                });
        });

        if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally chose either panels OR windows.");
            });
        }
    }
}

fn add_video(videos: &mut Vec<Video>, last_selected: &mut String) {
    let dir = videos_dir(&videos.to_vec())
        .unwrap_or_else(|| or(last_selected.to_string(), "/".to_string()));
    let target = rfd::FileDialog::new()
        .add_filter("Video files", &["avi", "mpg", "mp4", "mkv"])
        .set_directory(dir)
        .pick_file();

    match target {
        Some(t) => {
            let target_path = Path::new(&t);
            let file = target_path
                .to_str()
                .expect("Failed to convert input video path to string!")
                .to_string();
            println!("Adding video:{}", file);
            *last_selected = file.clone();
            let duration = video_duration(file.clone());
            videos.push(Video {
                index: videos.len(),
                file,
                scenes: vec![],
                duration,
            });
        }
        None => {}
    };
}

fn sync_scene_start(video: &mut Video, j: usize) {
    if j >= 1 {
        video.scenes[j - 1].end = video.scenes[j].start;
    }
}

fn sync_scene_end(video: &mut Video, j: usize) {
    if j + 1 < video.scenes.len() {
        video.scenes[j + 1].start = video.scenes[j].end;
    }
}

fn refresh_titles(
    instructional: &mut Instructional,
    candidate_urls: &mut Vec<String>,
    candidate_titles: &mut Vec<String>,
) {
    let instructionals: Vec<Instructional> =
        search_product(instructional.creator.to_string(), "".to_string());
    instructional.title = "".to_string();
    *candidate_urls = vec![];
    *candidate_titles = vec![];
    instructionals
        .iter()
        .map(|i| i.url.to_string())
        .for_each(|u| candidate_urls.push(u));
    instructionals
        .into_iter()
        .map(|i| i.title)
        .for_each(|t| candidate_titles.push(t));
}

fn create_ocr_image(
    frame: &epi::Frame,
    creator: String,
    title: String,
    s: &Scene,
) -> Option<egui::TextureId> {
    let ocr_img_path = scene_ocr_img_path(creator, title, s)?;
    let ocr_filename: String = ocr_img_path.to_str()?.to_string();
    load_texture_id(&frame, Path::new(&ocr_filename))
}

fn create_scene_image(
    frame: &epi::Frame,
    creator: String,
    title: String,
    s: &Scene,
) -> Option<egui::TextureId> {
    let img = scene_to_image(creator, title, s);
    img.map(|i| load_texture_id(&frame, Path::new(&i)).unwrap_or_default())
}

fn reallocate_scene_images(frame: &epi::Frame, videos: &Vec<Video>, v_index: usize, scene_images: Vec<Vec<Option<TextureId>>>) -> Vec<Vec<Option<egui::TextureId>>> {
    let mut result: Vec<Vec<Option<TextureId>>> = scene_images.clone();
    let video_scenes = videos[v_index].scenes.iter().map(|s| None).collect::<Vec<Option<egui::TextureId>>>();
    if v_index < scene_images.len() {
        result[v_index] = video_scenes;
        return result;
    }
    if v_index == scene_images.len() {
        result.insert(v_index, video_scenes);
        return result;
    }

    videos
        .iter()
        .map(|v| {
            v.scenes
                .iter()
                .map(|s| None)
                .collect::<Vec<Option<egui::TextureId>>>()
        })
        .collect::<Vec<Vec<Option<egui::TextureId>>>>()
}

fn allocate_scene_images(
    frame: &epi::Frame,
    videos: &Vec<Video>,
) -> Vec<Vec<Option<egui::TextureId>>> {
    videos
        .iter()
        .map(|v| {
            v.scenes
                .iter()
                .map(|s| None)
                .collect::<Vec<Option<egui::TextureId>>>()
        })
        .collect::<Vec<Vec<Option<egui::TextureId>>>>()
}

fn load_image(path: &Path) -> Option<epi::Image> {
    use image::GenericImageView;
    if !path.exists() {
        println!(
            "Image: {} does not exist!",
            path.as_os_str().to_str().unwrap().to_string()
        );
        return None;
    }
    let image = image::open(path).ok()?;
    let image_buffer = image.to_rgba();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image_buffer.into_vec();
    Some(epi::Image::from_rgba_unmultiplied(size, &pixels))
}

fn load_texture_id(frame: &epi::Frame, path: &Path) -> Option<egui::TextureId> {
    Some(frame.alloc_texture(load_image(path)?))
}

pub fn drag_source(ui: &mut egui::Ui, id: Id, body: impl FnOnce(&mut Ui)) {
    let is_being_dragged = ui.memory().is_being_dragged(id);

    if !is_being_dragged {
        let response = ui.scope(body).response;

        // Check for drags:
        let response = ui.interact(response.rect, id, Sense::drag());
        if response.hovered() {
            ui.output().cursor_icon = CursorIcon::Grab;
        }
    } else {
        ui.output().cursor_icon = CursorIcon::Grabbing;

        // Paint the body to a new layer:
        let layer_id = LayerId::new(Order::Tooltip, id);
        let response = ui.with_layer_id(layer_id, body).response;

        // Now we move the visuals of the body to where the mouse is.
        // Normally you need to decide a location for a widget first,
        // because otherwise that widget cannot interact with the mouse.
        // However, a dragged component cannot be interacted with anyway
        // (anything with `Order::Tooltip` always gets an empty `Response`)
        // So this is fine!

        if let Some(pointer_pos) = ui.input().pointer.interact_pos() {
            let delta = pointer_pos - response.rect.center();
            ui.ctx().translate_layer(layer_id, delta);
        }
    }
}

pub fn drop_target<R>(
    ui: &mut Ui,
    can_accept_what_is_being_dragged: bool,
    body: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let is_being_dragged = ui.memory().is_anything_being_dragged();

    let margin = Vec2::splat(4.0);

    let outer_rect_bounds = ui.available_rect_before_wrap();
    let inner_rect = outer_rect_bounds.shrink2(margin);
    let where_to_put_background = ui.painter().add(Shape::Noop);
    let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
    let ret = body(&mut content_ui);
    let outer_rect = Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);
    let (rect, response) = ui.allocate_at_least(outer_rect.size(), Sense::hover());

    let style = if is_being_dragged && can_accept_what_is_being_dragged && response.hovered() {
        ui.visuals().widgets.active
    } else {
        ui.visuals().widgets.inactive
    };

    let mut stroke = style.bg_stroke;
    if is_being_dragged && !can_accept_what_is_being_dragged {
        stroke.color = color::tint_color_towards(stroke.color, ui.visuals().window_fill());
    }

    ui.painter().set(
        where_to_put_background,
        epaint::RectShape {
            corner_radius: style.corner_radius,
            fill: ui.visuals().window_fill(),
            stroke,
            rect,
        },
    );

    InnerResponse::new(ret, response)
}

pub fn update_video_images(
    frame: &epi::Frame,
    sender: &mut Sender<Job>,
    v_index: usize,
    instructional: &mut Instructional,
) {
    println!("Updating video: {} images", v_index);
    for s_index in 0..instructional.videos[v_index].scenes.len() {
        update_scene_images(frame, sender, v_index, s_index, instructional)
    }
}

pub fn update_scene_images(
    frame: &epi::Frame,
    sender: &mut Sender<Job>,
    v_index: usize,
    s_index: usize,
    instructional: &mut Instructional,
) {
    println!("\tUpdating video: {}/{} images", v_index, s_index);
    sender
        .send(Job::CreateThumbnail {
            v_index,
            s_index,
            imageFn: create_scene_image,
        })
        .expect("Failed to send CreateThumbnail command!");
}

pub fn or(left: String, right: String) -> String {
    if !left.is_empty() && Path::new(&left).exists() {
        left
    } else {
        right
    }
}

pub fn scene_dir(scene: &Scene) -> Option<String> {
    if scene.file.is_empty() {
        None
    } else {
        parent_dir(&scene.file)
    }
}

pub fn scenes_dir(scenes: &Vec<Scene>) -> Option<String> {
    scenes
        .iter()
        .map(|s| scene_dir(s))
        .filter(|d| d.is_some())
        .map(|d| d.unwrap())
        .next()
}

pub fn videos_dir(videos: &Vec<Video>) -> Option<String> {
    videos
        .iter()
        .map(|v| video_dir(v))
        .filter(|d| d.is_some())
        .map(|d| d.unwrap())
        .next()
}

pub fn video_dir(video: &Video) -> Option<String> {
    if video.file.is_empty() {
        None
    } else {
        parent_dir(&video.file)
    }
}

pub fn parent_dir(file: &String) -> Option<String> {
    let path = Path::new(&file);
    if path.exists() {
        if path.is_dir() {
            Some(path.to_str().unwrap().to_string())
        } else {
            Some(path.parent().unwrap().to_str().unwrap().to_string())
        }
    } else {
        None
    }
}

fn get_icon(icon_name: &str) -> PathBuf {
    let local_icons = PathBuf::new()
        .join(env::current_dir().unwrap())
        .join("assets")
        .join("icons");
    if local_icons.exists() {
        return local_icons.join(icon_name);
    }

    return match env::var("HG2JJ_DIR") {
        Ok(d) => PathBuf::from(d)
            .join("assets")
            .join("icons")
            .join(icon_name),
        Err(_) => local_icons.join(icon_name),
    };
}

fn save_playlist_in_dir (instructional: &mut Instructional, playlist_file_name: String) {
    let video = instructional.videos.first().expect("Failed to find videos in instructional!");
    let target_file = PathBuf::from(&video.file).parent().expect("Failed to find part of file!").join(playlist_file_name);
    save_playlist(instructional, File::create(target_file.into_os_string()).expect("Failed to create file."));
}

fn save_org_in_dir (instructional: &mut Instructional, index_file_name: String) {
    let video = instructional.videos.first().expect("Failed to find videos in instructional!");
    let target_file = PathBuf::from(&video.file).parent().expect("Failed to find part of file!").join(index_file_name);
    save_org(instructional, File::create(target_file.into_os_string()).expect("Failed to create file."), false);
}

fn save_md_in_dir (instructional: &mut Instructional, index_file_name: String) {
    let video = instructional.videos.first().expect("Failed to find videos in instructional!");
    let target_file = PathBuf::from(&video.file).parent().expect("Failed to find part of file!").join(index_file_name);
    save_md(instructional, File::create(target_file.into_os_string()).expect("Failed to create file."));
}
