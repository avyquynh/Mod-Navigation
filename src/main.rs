    use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
    use askama::Template;
    use std::fs;
    use std::path::Path;
    
    #[derive(Template)]
    #[template(path = "file_view.html")]
    struct FileTemplate {
        name: String,
        path: String,
        file_type: String,
    }
    
    #[derive(Template)]
    #[template(path = "folder_view.html")]
    struct FolderTemplate {
        folder_name: String,
        items: Vec<FileSystemItem>,
    }
    
    struct FileSystemItem {
        name: String,
        path: String,
        is_folder: bool,
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
        render_folder("root_folder")
    }
    
    #[get("/folder/{tail:.*}")]
    async fn browse_folder(path: web::Path<String>) -> impl Responder {
        render_folder(&path.into_inner())
    }
    
    #[get("/file/{tail:.*}")]
    async fn view_file(path: web::Path<String>) -> impl Responder {
        let full_path = path.into_inner();
        let name = Path::new(&full_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        let template = FileTemplate {
            file_type: get_file_type(&full_path),
            path: format!("/static/{}", full_path),
            name,
        };
    
        match template.render() {
            Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
            Err(_) => HttpResponse::InternalServerError().body("Template Error"),
        }
    }
    
    fn render_folder(folder_path: &str) -> HttpResponse {
        let mut items = Vec::new();
        
        if let Ok(entries) = fs::read_dir(folder_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let rel_path = path.to_string_lossy().to_string();
                
                items.push(FileSystemItem {
                    name,
                    path: rel_path,
                    is_folder: path.is_dir(),
                });
            }
        }
    
        let template = FolderTemplate {
            folder_name: folder_path.to_string(),
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
                .service(actix_files::Files::new("/static", ".").show_files_listing())
                .service(index)
                .service(browse_folder)
                .service(view_file)
        })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
    }

