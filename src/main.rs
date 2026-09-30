    use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};    use askama::Template;
    use std::fs;
    use std::path::Path;
    
    #[derive(Template)]
    #[template(path = "file_view.html")]
    struct FileTemplate {
        name: String,
        path: String,
        file_type: String,
        file_name: String,
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
        if path.ends_with(".pdf") { "pdf".to_string() }
        else if path.ends_with(".mp4") { "video".to_string() }
        else if path.ends_with(".mp3") { "audio".to_string() }
        else if path.ends_with(".html") { "html".to_string() }
        else { "unknown".to_string() }
    }
    
    #[get("/")]
    async fn index() -> impl Responder {
        let topics = vec![
            Card::new("Arts", "arts.png", "#e8802a", "/folder/root_folder"),
            Card::new("Educational Tools", "educational-tools.png", "#c9a227", "/folder/root_folder"),
            Card::new("Health & Safety", "health-safety.png", "#c0392b", "/folder/root_folder"),
            Card::new("Information Literacy & Digital Literacy", "info-literacy.png", "#5a5a5a", "/folder/root_folder"),
            Card::new("Language & Reading", "language-reading.png", "#4a9c3f", "/folder/root_folder"),
            Card::new("Local Resources", "local-resources.png", "#a01f4a", "/folder/root_folder"),
            Card::new("Mathematics", "mathematics.png", "#6a4c93", "/folder/root_folder"),
            Card::new("Science", "science.png", "#e8b800", "/folder/root_folder"),
            Card::new("Social Studies", "social-studies.png", "#2a8fbd", "/folder/root_folder"),
            Card::new("Sustainability", "sustainability.png", "#2aa198", "/folder/root_folder"),
        ];

        let modules = vec![
            Card::new("SolarSPELL Training Course", "solarspell-training.png", "#2a8fbd", "/folder/root_folder"),
            Card::new("Global Health Media", "global-health-media.png", "#1a7fc4", "/folder/root_folder"),
            Card::new("Khan Academy", "khan-academy.png", "#2c2c2c", "/folder/root_folder"),
            Card::new("Medical Encyclopedia", "medical-encyclopedia.png", "#3a6b35", "/folder/root_folder"),
            Card::new("Science Activities", "science-activities.png", "#7fc4e0", "/folder/root_folder"),
            Card::new("Let's Learn English", "lets-learn-english.png", "#6a7bb5", "/folder/root_folder"),
            Card::new("Wikipedia for Schools", "wikipedia-schools.png", "#4aa8a0", "/folder/root_folder"),
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
        let web_path = if get_file_type(&file_path) == "pdf" {
            format!("/pdf/{}", file_path)
        } else {
            format!("/static/{}", file_path)
        };

        let template = FileTemplate {
            name: name.clone(),
            path: web_path,
            file_type,
            file_name: name,
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
    // Turn a folder/file name like "learning_materials" into "Learning Materials".
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

    fn render_folder(folder_path: &str) -> HttpResponse {
        if folder_path.trim_end_matches('/') != "root_folder" {
            return render_folder_list(folder_path);
        }

        let mut cards = Vec::new();

        if let Ok(entries) = fs::read_dir(folder_path) {
            let mut sorted: Vec<_> = entries.flatten().collect();
            sorted.sort_by_key(|e| e.file_name());

            for entry in sorted {
                let path = entry.path();
                let raw_name = path.file_name().unwrap().to_string_lossy().to_string();

                // Skip hidden files like .DS_Store
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

        // Build breadcrumb from the path segments.
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
    
    #[actix_web::main]
    async fn main() -> std::io::Result<()> {
        println!("Server running at http://127.0.0.1:8080");
    
        HttpServer::new(|| {
            App::new()
                .service(
                    actix_files::Files::new("/static", ".")
                    .show_files_listing()
                    .prefer_utf8(true)
                )
                .service(
                    actix_files::Files::new("/icons", "src/icons")
                    .prefer_utf8(true)
                )
                .service(
                    actix_files::Files::new("/header", "src/header")
                    .prefer_utf8(true)
                )
                .service(index)
                .service(browse_folder)
                .service(view_file)
                .service(serve_pdf) 
        })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
    }


