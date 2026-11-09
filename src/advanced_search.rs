use actix_web::{web, HttpResponse, Responder};
use askama::Template;
use rusqlite::{Connection, Result, types::ToSql};
use serde::Deserialize;
use std::{fmt, collections::HashMap};

use crate::get_db_connection_readonly;

#[derive(Debug)]
pub struct MetadataCategory {
    pub type_name: String,
    pub values: Vec<String>,
}

#[derive(Template)]
#[template(path = "search.html")]
#[allow(dead_code)] // total_results is set but the reference template never shows it
pub struct AdvancedSearchTemplate {
    list: Vec<AdvancedSearchResult>,
    categories: Vec<MetadataCategory>,
    total_results: u32,
    total_pages: u32,
    current_page: u32,
    prev_page: u32,
    next_page: u32,
    start_page: u32,
    end_page: u32,
    sort_by: String,
    sort_order: String,
    search_query: String,
    pub active_filters: HashMap<String, String>
}

#[derive(Deserialize)]
#[allow(dead_code)] // device_id/location_id were only used by the reference's usage logging
pub struct AdvancedSearchQuery {
    title: Option<String>,
    page: Option<u32>,
    sort_by: Option<String>,
    sort_order: Option<String>,
    device_id: Option<String>,
    location_id: Option<String>,
    #[serde(flatten)]
    metadata_filters: std::collections::HashMap<String, String>,
}

struct AdvancedSearchData {
    list: Vec<AdvancedSearchResult>,
    categories: Vec<MetadataCategory>,
    total_results: u32,
}

#[derive(Debug)]
pub struct AdvancedSearchResult {
    pub id: i64,
    pub title: String,
    pub file_type: String,
    pub size: f64,
}

#[derive(Debug)]
struct AdvancedSearchError(String);

impl fmt::Display for AdvancedSearchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AdvancedSearchError {}

fn get_all_categories_with_values(conn: &Connection) -> Result<Vec<MetadataCategory>> {
    println!("[DEBUG] Fetching all metadata categories with values");
    
    let mut type_stmt = conn.prepare(
        "SELECT DISTINCT type_name FROM metadata_type 
         WHERE type_name IS NOT NULL 
         ORDER BY type_name COLLATE NOCASE"
    )?;
    
    let type_names: Vec<String> = type_stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    
    let mut categories = Vec::new();
    
    for type_name in type_names {
        let mut value_stmt = conn.prepare(
            "SELECT DISTINCT meta_name FROM metadata 
             WHERE type_name = ?1 
             ORDER BY meta_name COLLATE NOCASE"
        )?;
        
        let values: Vec<String> = value_stmt
            .query_map([&type_name], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        
        categories.push(MetadataCategory {
            type_name,
            values,
        });
    }
    
    println!("[DEBUG] Found {} categories", categories.len());
    Ok(categories)
}

fn get_total_search_results(
    conn: &Connection, 
    search_query: &str,
    metadata_filters: &std::collections::HashMap<String, String>,
) -> Result<u32> {
    println!("[DEBUG] Counting total search results");

    let raw_q = search_query.trim();

    let fts_match_q = if !raw_q.is_empty() {
        let clean: String = raw_q
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect();
        let tokens: Vec<String> = clean
            .split_whitespace()
            .filter(|t| !t.is_empty())
            .map(|t| format!("{}*", t))
            .collect();
        if tokens.is_empty() {
            None
        } else {
            Some(tokens.join(" "))
        }
    } else {
        None
    };

    let escaped_q = raw_q.replace('%', "\\%").replace('_', "\\_");

    let mut query = "SELECT COUNT(DISTINCT T1.id) FROM content AS T1".to_string();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref fts_q) = fts_match_q {
        query.push_str(" LEFT JOIN docs ON docs.rowid = T1.id AND docs MATCH ?");
        params.push(Box::new(fts_q.clone()));
    }

    let mut join_idx = 2; 
    for (_, _) in metadata_filters.iter().filter(|(_, v)| !v.is_empty()) {
        query.push_str(&format!(
            " INNER JOIN content_metadata AS CM{} ON T1.id = CM{}.content_id \
             INNER JOIN metadata AS M{} ON CM{}.metadata_id = M{}.id",
            join_idx, join_idx, join_idx, join_idx, join_idx
        ));
        join_idx += 1;
    }

    let mut where_clauses = Vec::new();

    if let Some(_) = fts_match_q {
        where_clauses.push("(T1.title LIKE ? OR docs.rowid IS NOT NULL)".to_string());
        params.push(Box::new(format!("%{}%", escaped_q)));
    } else if !raw_q.is_empty() {
        where_clauses.push("T1.title LIKE ?".to_string());
        params.push(Box::new(format!("%{}%", escaped_q)));
    } else {
        where_clauses.push("1=1".to_string());
    }

    let mut meta_idx = 2; 
    for (type_name, value) in metadata_filters.iter().filter(|(_, v)| !v.is_empty()) {
        where_clauses.push(format!(
            "(M{}.type_name = ? AND M{}.meta_name = ?)",
            meta_idx, meta_idx
        ));
        params.push(Box::new(type_name.clone()));
        params.push(Box::new(value.clone()));
        meta_idx += 1;
    }

    if !where_clauses.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&where_clauses.join(" AND "));
    }

    println!("[DEBUG] Count query: {}", query);

    let mut stmt = conn.prepare(&query)?;
    let count: u32 = stmt.query_row(
        rusqlite::params_from_iter(params.iter().map(|p| &**p)),
        |row| row.get(0),
    )?;

    println!("[DEBUG] Total results found: {}", count);
    Ok(count)
}

fn get_advanced_search_data(
    search_query: &str,
    page: u32,
    sort_by: &str,
    sort_order: &str,
    metadata_filters: &std::collections::HashMap<String, String>,
) -> Result<AdvancedSearchData, Box<dyn std::error::Error>> {
    println!(
        "[DEBUG] Enter get_advanced_search_data: page={}, sort_by='{}', sort_order='{}'",
        page, sort_by, sort_order
    );
    let conn = get_db_connection_readonly()?;
    let total_results = get_total_search_results(&conn, search_query, metadata_filters)?;
    
    let list = get_advanced_search_result(&conn, search_query, page, sort_by, sort_order, metadata_filters)?;
    let categories = get_all_categories_with_values(&conn)?;

    conn.close().map_err(|(_, e)| {
        println!("[ERROR] Error closing database connection: {}", e);
        Box::new(AdvancedSearchError(e.to_string())) as Box<dyn std::error::Error>
    })?;

    println!("[DEBUG] get_advanced_search_data() completed successfully.");
    Ok(AdvancedSearchData {
        list,
        categories,
        total_results,
    })
}

pub fn get_advanced_search_result(
    conn: &Connection,
    search_terms: &str,
    page: u32,
    _sort_by: &str,
    _sort_order: &str,
    metadata_filters: &HashMap<String, String>,
) -> Result<Vec<AdvancedSearchResult>> {
    println!("[DEBUG] Running unified search with page={}", page);

    let page_size: i64 = 25;
    let safe_page = if page == 0 { 1 } else { page };
    let offset: i64 = ((safe_page - 1) as i64) * page_size;
    let raw_q = search_terms.trim();

    let escaped_q = raw_q.replace('%', "\\%").replace('_', "\\_");

    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    let fts_match_q = if !raw_q.is_empty() {
        let clean: String = raw_q.chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect();
        let tokens: Vec<String> = clean.split_whitespace()
            .filter(|t| !t.is_empty())
            .map(|t| format!("{}*", t))
            .collect();
        if tokens.is_empty() { None } else { Some(tokens.join(" ")) }
    } else {
        None
    };

    let mut query = String::from("SELECT DISTINCT T1.id, T1.title, T1.file_size AS size, T3.meta_name AS file_type, ");
    
    query.push_str("(lower(T1.title) = lower(?)) AS exact_match, ");
    params.push(Box::new(raw_q.to_string()));

    query.push_str("(lower(T1.title) LIKE lower(?) || '%') AS prefix_match, ");
    params.push(Box::new(raw_q.to_string()));

    if fts_match_q.is_some() {
        query.push_str("(docs.rowid IS NOT NULL) AS fts_match ");
    } else {
        query.push_str("0 AS fts_match ");
    }

    query.push_str("FROM content AS T1 ");

    if let Some(ref fts_q) = fts_match_q {
        query.push_str("LEFT JOIN docs ON docs.rowid = T1.id AND docs MATCH ? ");
        params.push(Box::new(fts_q.clone()));
    }

    query.push_str("LEFT JOIN content_metadata AS T2 ON T1.id = T2.content_id \
                    LEFT JOIN metadata AS T3 ON T2.metadata_id = T3.id AND T3.type_name = 'file_type' ");

    let mut join_idx = 10; 
    for (_, _) in metadata_filters.iter().filter(|(_, v)| !v.is_empty()) {
        query.push_str(&format!(
            "INNER JOIN content_metadata AS CM{0} ON T1.id = CM{0}.content_id \
             INNER JOIN metadata AS M{0} ON CM{0}.metadata_id = M{0}.id ",
            join_idx
        ));
        join_idx += 1;
    }

    if fts_match_q.is_some() {
        query.push_str(" WHERE (T1.title LIKE ? OR docs.rowid IS NOT NULL)");
        params.push(Box::new(format!("%{}%", escaped_q)));
    } else if !raw_q.is_empty() {
        query.push_str(" WHERE T1.title LIKE ?");
        params.push(Box::new(format!("%{}%", escaped_q)));
    } else {
        query.push_str(" WHERE 1=1");
    }

    // Metadata filter values
    join_idx = 10;
    for (type_name, val) in metadata_filters.iter().filter(|(_, v)| !v.is_empty()) {
        query.push_str(&format!(" AND M{}.type_name = ? AND M{}.meta_name = ?", join_idx, join_idx));
        params.push(Box::new(type_name.clone()));
        params.push(Box::new(val.clone()));
        join_idx += 1;
    }

    query.push_str(" ORDER BY exact_match DESC, prefix_match DESC, fts_match DESC, T1.title ASC");
    query.push_str(" LIMIT ? OFFSET ?");
    params.push(Box::new(page_size));
    params.push(Box::new(offset));

    let mut stmt = conn.prepare(&query)?;
    let mapped = stmt.query_map(rusqlite::params_from_iter(params.iter().map(|p| &**p)), |row| {
        Ok(AdvancedSearchResult {
            id: row.get(0)?,
            title: row.get(1)?,
            size: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
            file_type: row.get::<_, Option<String>>(3)?.unwrap_or_else(|| "unknown".to_string()),
        })
    })?;

    let mut results = Vec::new();
    for r in mapped {
        results.push(r?);
    }
    Ok(results)
}

impl AdvancedSearchTemplate {
    fn make_url(&self, page: u32) -> String {
        let mut params = format!("title={}&page={}&sort_by={}&sort_order={}", 
            self.search_query, page, self.sort_by, self.sort_order);
        
        for (key, val) in &self.active_filters {
            if !val.is_empty() {
                params.push_str(&format!("&{}={}", key, val));
            }
        }
        params
    }
}

pub async fn advanced_search(form: web::Query<AdvancedSearchQuery>) -> impl Responder {
    let search_query = form.title.as_deref().unwrap_or("").to_string();
    let page = form.page.unwrap_or(1).max(1);
    let sort_by = form.sort_by.as_deref().unwrap_or("rank");
    let sort_order = form.sort_order.as_deref().unwrap_or("asc");
    
    let mut metadata_filters = form.metadata_filters.clone();
    metadata_filters.remove("title");
    metadata_filters.remove("page");
    metadata_filters.remove("sort_by");
    metadata_filters.remove("sort_order");
    metadata_filters.remove("device_id");
    metadata_filters.remove("location_id");
    
    println!("[DEBUG] Metadata filters: {:?}", metadata_filters);

    let data = match get_advanced_search_data(&search_query, page, sort_by, sort_order, &metadata_filters) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[ERROR] Failed to get advanced search data: {}", e);
            return HttpResponse::InternalServerError().body("Failed to load search results");
        }
    };

    let total_pages = std::cmp::max(1, ((data.total_results as f64) / 25.0).ceil() as u32);
    let prev_page = if page > 1 { page - 1 } else { 1 };
    let next_page = if page < total_pages { page + 1 } else { total_pages };
    let start_page = if page > 3 { page - 2 } else { 1 };
    let end_page = std::cmp::min(page + 2, total_pages);

    println!(
        "[DEBUG] Pagination values → prev_page={}, current_page={}, next_page={}, start_page={}, end_page={}, total_pages={}",
        prev_page, page, next_page, start_page, end_page, total_pages
    );

    let template = AdvancedSearchTemplate {
        list: data.list,
        categories: data.categories,
        total_results: data.total_results,
        total_pages,
        current_page: page,
        prev_page,
        next_page,
        start_page,
        end_page,
        sort_by: sort_by.to_string(),
        sort_order: sort_order.to_string(),
        search_query,
        active_filters: metadata_filters,
    };

    println!(
        "[DEBUG] Rendering template with current_page={}, prev_page={}, next_page={}, total_pages={}",
        template.current_page, template.prev_page, template.next_page, template.total_pages
    );

    match template.render() {
        Ok(body) => {
            println!("[DEBUG] Template rendered successfully, returning HTML response.");
            HttpResponse::Ok().content_type("text/html").body(body)
        }
        Err(e) => {
            eprintln!("[ERROR] Error rendering advanced_search template: {}", e);
            HttpResponse::InternalServerError().into()
        }
    }
}
