use actix_web::{get, web, App, Error, HttpResponse, HttpServer, Responder};
use askama::Template;
use rusqlite::{Connection, OpenFlags};
use std::fs;
use std::path::Path;

mod advanced_search;

fn get_db_connection_readonly() -> rusqlite::Result<Connection> {
    Connection::open_with_flags("solarspell.db", OpenFlags::SQLITE_OPEN_READ_ONLY)
}

#[derive(Template)]
#[template(path = "file_view.html")]
struct PathFileTemplate {
    name: String,
    path: String,
    file_type: String,
    type_label: String,
    size: String,
    location: String,
    crumbs: Vec<Crumb>,
}

#[derive(Template)]
#[template(path = "folder_view.html")]
struct FolderTemplate {
    title: String,
    crumbs: Vec<Crumb>,
    cards: Vec<FolderCard>,
}

struct Crumb {
    name: String,
    link: String,
    last: bool,
}

struct FolderCard {
    name: String,
    tag: String,
    description: String,
    link: String,
}

#[derive(Template)]
#[template(path = "folder_list.html")]
struct FolderListTemplate {
    title: String,
    items: Vec<FileSystemItem>,
}

struct FileSystemItem {
    name: String,
    path: String,
    is_folder: bool,
}

#[derive(Template)]
#[template(path = "materials_view.html")]
struct MaterialsTemplate {
    title: String,
    crumbs: Vec<Crumb>,
    sections: Vec<MaterialSection>,
}

struct MaterialSection {
    number: String,
    name: String,
    open: bool,
    files: Vec<MaterialFile>,
}

struct MaterialFile {
    name: String,
    link: String,
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    topics: Vec<Card>,
    modules: Vec<Card>,
}

struct Card {
    name: String,
    icon: String,
    color: String,
    link: String,
}

impl Card {
    fn new(name: &str, icon: &str, color: &str, link: &str) -> Self {
        Card {
            name: name.to_string(),
            icon: icon.to_string(),
            color: color.to_string(),
            link: link.to_string(),
        }
    }
}

fn get_file_type(path: &str) -> String {
    let path = path.to_lowercase();
    if path.ends_with(".pdf") {
        "pdf".to_string()
    } else if path.ends_with(".mp4") {
        "video".to_string()
    } else if path.ends_with(".mp3") {
        "audio".to_string()
    } else if path.ends_with(".html") {
        "html".to_string()
    } else {
        "unknown".to_string()
    }
}

#[get("/")]
async fn index() -> impl Responder {
    let topics = vec![
        Card::new("Arts", "arts.png", "#e8802a", "/folder/root_folder"),
        Card::new(
            "Educational Tools",
            "educational-tools.png",
            "#c9a227",
            "/folder/root_folder",
        ),
        Card::new(
            "Health & Safety",
            "health-safety.png",
            "#c0392b",
            "/folder/root_folder",
        ),
        Card::new(
            "Information Literacy & Digital Literacy",
            "info-literacy.png",
            "#5a5a5a",
            "/folder/root_folder",
        ),
        Card::new(
            "Language & Reading",
            "language-reading.png",
            "#4a9c3f",
            "/folder/root_folder",
        ),
        Card::new(
            "Local Resources",
            "local-resources.png",
            "#a01f4a",
            "/folder/root_folder",
        ),
        Card::new(
            "Mathematics",
            "mathematics.png",
            "#6a4c93",
            "/folder/root_folder",
        ),
        Card::new("Science", "science.png", "#e8b800", "/folder/root_folder"),
        Card::new(
            "Social Studies",
            "social-studies.png",
            "#2a8fbd",
            "/folder/root_folder",
        ),
        Card::new(
            "Sustainability",
            "sustainability.png",
            "#2aa198",
            "/folder/root_folder",
        ),
    ];

    let modules = vec![
        Card::new(
            "SolarSPELL Training Course",
            "solarspell-training.png",
            "#2a8fbd",
            "/folder/root_folder",
        ),
        Card::new(
            "Global Health Media",
            "global-health-media.png",
            "#1a7fc4",
            "/folder/root_folder",
        ),
        Card::new(
            "Khan Academy",
            "khan-academy.png",
            "#2c2c2c",
            "/folder/root_folder",
        ),
        Card::new(
            "Medical Encyclopedia",
            "medical-encyclopedia.png",
            "#3a6b35",
            "/folder/root_folder",
        ),
        Card::new(
            "Science Activities",
            "science-activities.png",
            "#7fc4e0",
            "/folder/root_folder",
        ),
        Card::new(
            "Let's Learn English",
            "lets-learn-english.png",
            "#6a7bb5",
            "/folder/root_folder",
        ),
        Card::new(
            "Wikipedia for Schools",
            "wikipedia-schools.png",
            "#4aa8a0",
            "/folder/root_folder",
        ),
    ];

    let template = HomeTemplate { topics, modules };
    match template.render() {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(_) => HttpResponse::InternalServerError().body("Template Error"),
    }
}

#[get("/folder/{tail:.*}")]
async fn browse_folder(path: web::Path<String>) -> impl Responder {
    render_folder(&path.into_inner())
}

#[get("/file/{path:.*}")]
async fn view_file(path: web::Path<String>) -> impl Responder {
    let file_path = path.into_inner();

    let name = Path::new(&file_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let file_type = get_file_type(&file_path);
    let encoded = encode_path(&file_path);
    let web_path = if file_type == "pdf" {
        format!("/pdf/{}", encoded)
    } else {
        format!("/static/{}", encoded)
    };

    let size = fs::metadata(&file_path)
        .map(|m| format_size(m.len()))
        .unwrap_or_default();

    let parent = Path::new(&file_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // Breadcrumbs: every folder is a link, and the file itself is the current page.
    let (mut crumbs, _) = build_crumbs(&parent);
    for crumb in crumbs.iter_mut() {
        crumb.last = false;
    }
    crumbs.push(Crumb {
        name: name.clone(),
        link: String::new(),
        last: true,
    });

    let type_label = match file_type.as_str() {
        "pdf" => "PDF",
        "video" => "Video",
        "audio" => "Audio",
        "html" => "HTML",
        _ => "File",
    }
    .to_string();

    let template = PathFileTemplate {
        name,
        path: web_path,
        file_type,
        type_label,
        size,
        location: prettify_path(&parent),
        crumbs,
    };

    match template.render() {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(_) => HttpResponse::InternalServerError().body("Template Error"),
    }
}

#[get("/pdf/{path:.*}")]
async fn serve_pdf(path: web::Path<String>) -> impl Responder {
    let file_path = path.into_inner();

    match fs::read(&file_path) {
        Ok(bytes) => HttpResponse::Ok()
            .content_type("application/pdf")
            .insert_header(("Content-Disposition", "inline"))
            .body(bytes),
        Err(_) => HttpResponse::NotFound().body("File not found"),
    }
}
fn prettify(name: &str) -> String {
    name.split(|c| c == '_' || c == '-' || c == ' ')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// "root_folder/learning_materials" -> "Root Folder / Learning Materials"
fn prettify_path(path: &str) -> String {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(prettify)
        .collect::<Vec<_>>()
        .join(" / ")
}

// Build a Home > ... > current breadcrumb for a folder path.
fn build_crumbs(folder_path: &str) -> (Vec<Crumb>, String) {
    let mut crumbs = vec![Crumb {
        name: "Home".to_string(),
        link: "/".to_string(),
        last: false,
    }];

    let segments: Vec<&str> = folder_path.split('/').filter(|s| !s.is_empty()).collect();
    let mut accumulated = String::new();
    for (i, seg) in segments.iter().enumerate() {
        if !accumulated.is_empty() {
            accumulated.push('/');
        }
        accumulated.push_str(seg);
        crumbs.push(Crumb {
            name: prettify(seg),
            link: format!("/folder/{}", accumulated),
            last: i == segments.len() - 1,
        });
    }

    let title = segments
        .last()
        .map(|s| prettify(s))
        .unwrap_or_else(|| "Library".to_string());

    (crumbs, title)
}

fn render_folder(folder_path: &str) -> HttpResponse {
    let trimmed = folder_path.trim_end_matches('/');

    if trimmed == "root_folder" {
        // falls through to the card layout below
    } else if trimmed.ends_with("learning_materials") {
        return render_materials(folder_path);
    } else {
        return render_folder_list(folder_path);
    }

    let mut cards = Vec::new();

    if let Ok(entries) = fs::read_dir(folder_path) {
        let mut sorted: Vec<_> = entries.flatten().collect();
        sorted.sort_by_key(|e| e.file_name());

        for entry in sorted {
            let path = entry.path();
            let raw_name = path.file_name().unwrap().to_string_lossy().to_string();

            if raw_name.starts_with('.') {
                continue;
            }

            let rel_path = path.to_string_lossy().to_string();
            let is_folder = path.is_dir();

            let (tag, description) = if is_folder {
                (
                    "Folder".to_string(),
                    "Open this folder to explore its lessons and materials.".to_string(),
                )
            } else {
                let ftype = get_file_type(&rel_path);
                let tag = match ftype.as_str() {
                    "pdf" => "PDF",
                    "video" => "Video",
                    "audio" => "Audio",
                    "html" => "Lesson",
                    _ => "File",
                };
                (
                    tag.to_string(),
                    format!("Open this {} resource.", tag.to_lowercase()),
                )
            };

            let link = if is_folder {
                format!("/folder/{}", rel_path)
            } else {
                format!("/file/{}", rel_path)
            };

            cards.push(FolderCard {
                name: prettify(&raw_name),
                tag,
                description,
                link,
            });
        }
    }

    let (crumbs, title) = build_crumbs(folder_path);

    let template = FolderTemplate {
        title,
        crumbs,
        cards,
    };

    match template.render() {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(_) => HttpResponse::InternalServerError().body("Template Error"),
    }
}

// Accordion layout for learning_materials: each subfolder becomes a
// collapsible section, and the files inside become downloadable rows.
// Any files sitting directly in learning_materials are grouped into their
// own "Materials" section.
fn render_materials(folder_path: &str) -> HttpResponse {
    let mut sections: Vec<MaterialSection> = Vec::new();
    let mut loose_files: Vec<MaterialFile> = Vec::new();

    if let Ok(entries) = fs::read_dir(folder_path) {
        let mut sorted: Vec<_> = entries.flatten().collect();
        sorted.sort_by_key(|e| e.file_name());

        for entry in sorted {
            let path = entry.path();
            let raw_name = path.file_name().unwrap().to_string_lossy().to_string();

            if raw_name.starts_with('.') {
                continue;
            }

            let rel_path = path.to_string_lossy().to_string();

            if path.is_dir() {
                // Collect files inside this subfolder.
                let mut files = Vec::new();
                if let Ok(sub_entries) = fs::read_dir(&path) {
                    let mut sub_sorted: Vec<_> = sub_entries.flatten().collect();
                    sub_sorted.sort_by_key(|e| e.file_name());

                    for sub in sub_sorted {
                        let sub_path = sub.path();
                        let sub_name = sub_path.file_name().unwrap().to_string_lossy().to_string();

                        if sub_name.starts_with('.') || sub_path.is_dir() {
                            continue;
                        }

                        files.push(MaterialFile {
                            name: sub_name,
                            link: format!("/file/{}", sub_path.to_string_lossy()),
                        });
                    }
                }

                sections.push(MaterialSection {
                    number: String::new(), // numbered after collection
                    name: prettify(&raw_name),
                    open: false,
                    files,
                });
            } else {
                loose_files.push(MaterialFile {
                    name: raw_name,
                    link: format!("/file/{}", rel_path),
                });
            }
        }
    }

    // Loose files (directly inside learning_materials) go into their own section at the top.
    if !loose_files.is_empty() {
        sections.insert(
            0,
            MaterialSection {
                number: String::new(),
                name: "Materials".to_string(),
                open: false,
                files: loose_files,
            },
        );
    }

    // Number the sections (01, 02, ...) and open the first by default.
    for (i, section) in sections.iter_mut().enumerate() {
        section.number = format!("{:02}", i + 1);
        section.open = i == 0;
    }

    let (crumbs, title) = build_crumbs(folder_path);

    let template = MaterialsTemplate {
        title,
        crumbs,
        sections,
    };

    match template.render() {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(_) => HttpResponse::InternalServerError().body("Template Error"),
    }
}

fn render_folder_list(folder_path: &str) -> HttpResponse {
    let mut items = Vec::new();

    if let Ok(entries) = fs::read_dir(folder_path) {
        let mut sorted: Vec<_> = entries.flatten().collect();
        sorted.sort_by_key(|e| e.file_name());

        for entry in sorted {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();

            if name.starts_with('.') {
                continue;
            }

            let rel_path = path.to_string_lossy().to_string();

            items.push(FileSystemItem {
                name,
                path: rel_path,
                is_folder: path.is_dir(),
            });
        }
    }

    let template = FolderListTemplate {
        title: folder_path.to_string(),
        items,
    };

    match template.render() {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(_) => HttpResponse::InternalServerError().body("Template Error"),
    }
}

// Percent-encode each path segment but keep the slashes, so names with spaces or '#' load.
fn encode_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            let mut out = String::new();
            for b in segment.bytes() {
                match b {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        out.push(b as char)
                    }
                    _ => out.push_str(&format!("%{:02X}", b)),
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}

// File (database item, linked from search) -------------------------------------------------------
// Ported from the reference repo (solarspell-app-rust, naifs_branch).
#[derive(Template)]
#[template(path = "file.html")]
struct FileTemplate {
    data: File,
    meta: Metadata,
}

// The reference also builds `parent_path` and `lower` here, but only its analytics script
// reads them, and this app has no analytics endpoint.
fn get_file_template(file_id: i32) -> Result<FileTemplate, Error> {
    let conn = get_db_connection_readonly().map_err(|e| {
        eprintln!("Database connection error: {:?}", e);
        actix_web::error::ErrorInternalServerError("DB connection failed")
    })?;

    // An unknown id returns an error (500) instead of panicking like the reference.
    let data = get_file(&conn, file_id).map_err(actix_web::error::ErrorInternalServerError)?;
    let meta =
        metadata_by_file(&conn, file_id).map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(FileTemplate { data, meta })
}

#[allow(dead_code)]
struct File {
    file_id: i32,
    title: String,
    description: String,
    file_name: String,
    file_type: String,
    published_date: String,
    copyright_notes: String,
    rights_statement: String,
    file_size: f32,
}

fn get_file(conn: &Connection, file_id: i32) -> rusqlite::Result<File> {
    let mut stmt = conn.prepare(
        "SELECT id, title, description, file_name, published_date,
	copyright_notes, rights_statement, file_size FROM content WHERE id = ?1",
    )?;

    stmt.query_row(rusqlite::params![file_id], |row| {
        let description_option: Option<String> = row.get(2)?;
        let description = description_option.unwrap_or_default();

        let published_date_option: Option<String> = row.get(4)?;
        let published_date = match published_date_option {
            Some(date) => date.get(0..4).unwrap_or_default().to_string(),
            None => "".to_string(),
        };

        let copyright_notes_option: Option<String> = row.get(5)?;
        let copyright_notes = copyright_notes_option.unwrap_or_default();

        let rights_statement_option: Option<String> = row.get(6)?;
        let rights_statement = rights_statement_option.unwrap_or_default();

        let file_name: String = row.get(3)?;

        Ok(File {
            file_id: row.get(0)?,
            title: row.get(1)?,
            description,
            file_type: get_file_extension(&file_name),
            file_name,
            published_date,
            copyright_notes,
            rights_statement,
            file_size: row.get(7)?,
        })
    })
}

// Returns the last three letters of the file name (e.g. "pdf", "mp4").
// Named differently from get_file_type above, which handles the root_folder pages.
fn get_file_extension(file_name: &str) -> String {
    file_name
        .get(file_name.len().saturating_sub(3)..)
        .unwrap_or_default()
        .to_string()
}

async fn file(path_id: web::Path<i32>) -> impl Responder {
    let id = path_id.into_inner();
    let template = match get_file_template(id) {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    match template.render() {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[derive(Default)]
struct Metadata {
    language: String,
    keywords: Vec<String>,
    creators: Vec<String>,
    resource_types: Vec<String>,
    audience: Vec<String>,
    subjects: Vec<String>,
    format: String,
    rights_holder: String,
}

// Pulls all metadata values and their type ids, then sorts them into the matching Metadata field.
fn metadata_by_file(conn: &Connection, file_id: i32) -> rusqlite::Result<Metadata> {
    let mut stmt = conn.prepare(
        "
	SELECT metadata.type_id, metadata.meta_name
	FROM content_metadata JOIN metadata
	ON content_metadata.metadata_id = metadata.id
	WHERE content_metadata.content_id = ?1
	ORDER BY metadata.type_id, metadata.meta_name",
    )?;

    let file_metadata = stmt.query_map(rusqlite::params![file_id], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;

    let mut meta = Metadata::default();

    for data in file_metadata {
        let (type_id, meta_name): (i32, String) = data?;
        match type_id {
            1 => meta.language = meta_name,
            3 => meta.keywords.push(meta_name),
            5 => meta.creators.push(meta_name),
            6 => meta.resource_types.push(meta_name),
            7 => meta.audience.push(meta_name),
            9 => meta.subjects.push(meta_name),
            11 => meta.format = meta_name,
            12 => meta.rights_holder = meta_name,
            _ => {}
        }
    }

    Ok(meta)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Server running at http://127.0.0.1:8080");

    HttpServer::new(|| {
        App::new()
            .service(
                actix_files::Files::new("/static", ".")
                    .show_files_listing()
                    .prefer_utf8(true),
            )
            .service(actix_files::Files::new("/icons", "src/icons").prefer_utf8(true))
            .service(actix_files::Files::new("/header", "src/header").prefer_utf8(true))
            .service(actix_files::Files::new("/assets", "static").prefer_utf8(true))
            .service(index)
            .route("/search", web::get().to(advanced_search::advanced_search))
            .route(
                "/advanced_search",
                web::get().to(advanced_search::advanced_search),
            )
            // Numeric ids are database items from search; must come before the path-based /file route.
            .route("/file/{id:\\d+}", web::get().to(file))
            .service(browse_folder)
            .service(view_file)
            .service(serve_pdf)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
