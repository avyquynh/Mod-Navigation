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
                .service(
                    actix_files::Files::new("/static", ".")
                    .show_files_listing()
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


