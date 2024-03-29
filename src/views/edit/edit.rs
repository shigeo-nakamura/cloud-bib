use crate::error::BibErrorResponse;
use crate::item::{atoi, insert_item, search_items, SystemSetting};
use crate::item::{delete_item, search_item, update_item};
use crate::item::{Book, User};
use crate::views::cache::Cache;
use crate::views::content_loader::read_file;
use crate::views::db_helper::get_db;
use crate::views::reply::Reply;
use crate::views::session::check_operator_session;
use actix_session::Session;
use actix_web::{web, HttpResponse, Result};
use log::debug;
use serde::Deserialize;
use shared_mongodb::ClientHolder;
use std::collections::HashMap;
use std::sync::Mutex;

pub async fn load(_session: Session) -> HttpResponse {
    let html_data = read_file("src/html/edit.html").unwrap();
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html_data)
}

#[derive(Deserialize, Debug)]
pub struct UpdateUserForm {
    pub user_id: String,
    pub user_name: String,
    pub user_kana: String,
    pub user_category: String,
    pub user_grade: String,
    pub user_remark: String,
    pub user_register_date: String,
}

pub async fn insert_user(
    session: Session,
    form: web::Json<UpdateUserForm>,
    data: web::Data<Mutex<ClientHolder>>,
    setting_map: web::Data<Mutex<HashMap<String, SystemSetting>>>,
) -> Result<HttpResponse, BibErrorResponse> {
    debug!("{:?}", form);
    user(&session, &form, &data, &setting_map, "insert").await
}

pub async fn update_user(
    session: Session,
    form: web::Json<UpdateUserForm>,
    data: web::Data<Mutex<ClientHolder>>,
    setting_map: web::Data<Mutex<HashMap<String, SystemSetting>>>,
) -> Result<HttpResponse, BibErrorResponse> {
    debug!("{:?}", form);
    user(&session, &form, &data, &setting_map, "update").await
}

pub async fn delete_user(
    session: Session,
    form: web::Json<UpdateUserForm>,
    data: web::Data<Mutex<ClientHolder>>,
    setting_map: web::Data<Mutex<HashMap<String, SystemSetting>>>,
) -> Result<HttpResponse, BibErrorResponse> {
    debug!("{:?}", form);
    user(&session, &form, &data, &setting_map, "delete").await
}

async fn user(
    session: &Session,
    form: &web::Json<UpdateUserForm>,
    data: &web::Data<Mutex<ClientHolder>>,
    setting_map: &web::Data<Mutex<HashMap<String, SystemSetting>>>,
    operation: &str,
) -> Result<HttpResponse, BibErrorResponse> {
    let dbname = check_operator_session(&session)?;
    let db = get_db(&data, &session).await?;

    let setting_map = setting_map.lock().unwrap();
    let setting = setting_map.get(&dbname);
    if setting.is_none() {
        return Err(BibErrorResponse::NotAuthorized);
    }
    let setting = setting.unwrap().clone();
    drop(setting_map);

    // Read the User from DB first
    let mut user = User::default();
    user.id = atoi(&form.user_id).map_err(|e| BibErrorResponse::InvalidArgument(e.to_string()))?;
    user = match search_item(&db, &user).await {
        Ok(mut user) => {
            if operation == "insert" {
                return Err(BibErrorResponse::ItemAlreadyExists(user.id));
            }
            user.name = form.user_name.clone();
            user.kana = form.user_kana.clone();
            user.category = form.user_category.clone();
            user.grade = form.user_grade.clone();
            user.remark = form.user_remark.clone();
            user.register_date = form.user_register_date.clone();
            user
        }
        Err(_) => {
            // Check the number of items
            user.id = 0;
            let users = match search_items(&db, &user).await {
                Ok(users) => users,
                Err(_) => vec![],
            };
            let nsize: u32 = users.len().try_into().unwrap();
            if nsize == setting.max_registered_users {
                return Err(BibErrorResponse::ExceedLimit(nsize));
            }

            User::new(
                &form.user_id,
                &form.user_name,
                &form.user_kana,
                &form.user_category,
                &form.user_grade,
                &form.user_remark,
                &form.user_register_date,
            )
            .unwrap()
        }
    };

    match operation {
        "insert" => {
            insert_item(&db, &user)
                .await
                .map_err(|e| BibErrorResponse::SystemError(e.to_string()))?;
        }
        "update" => {
            update_item(&db, &user)
                .await
                .map_err(|e| BibErrorResponse::SystemError(e.to_string()))?;
        }
        "delete" => {
            if user.borrowed_books.len() > 0 {
                return Err(BibErrorResponse::NotPossibleToDelete);
            }
            delete_item(&db, &user)
                .await
                .map_err(|e| BibErrorResponse::SystemError(e.to_string()))?;
        }
        _ => {
            return Err(BibErrorResponse::NotImplemented);
        }
    }

    let reply = Reply::default();
    Ok(HttpResponse::Ok().json(reply))
}

#[derive(Deserialize, Debug)]
pub struct UpdateBookForm {
    pub book_id: String,
    pub book_title: String,
    pub book_location: String,
    pub book_category: String,
    pub book_status: String,
    pub book_author: String,
    pub book_publisher: String,
    pub book_published_date: String,
    pub book_series: String,
    pub book_volume: String,
    pub book_page: String,
    pub book_kana: String,
    pub book_category_symbol: String,
    pub book_library_symbol: String,
    pub book_volume_symbol: String,
    pub book_forbidden: String,
    pub book_remark: String,
    pub book_isbn: String,
    pub book_register_date: String,
    pub book_register_type: String,
}

pub async fn insert_book(
    session: Session,
    form: web::Json<UpdateBookForm>,
    data: web::Data<Mutex<ClientHolder>>,
    cache_map: web::Data<Mutex<HashMap<String, Cache>>>,
    setting_map: web::Data<Mutex<HashMap<String, SystemSetting>>>,
) -> Result<HttpResponse, BibErrorResponse> {
    debug!("{:?}", form);
    book(&session, &form, &data, &cache_map, &setting_map, "insert").await
}

pub async fn update_book(
    session: Session,
    form: web::Json<UpdateBookForm>,
    data: web::Data<Mutex<ClientHolder>>,
    cache_map: web::Data<Mutex<HashMap<String, Cache>>>,
    setting_map: web::Data<Mutex<HashMap<String, SystemSetting>>>,
) -> Result<HttpResponse, BibErrorResponse> {
    debug!("{:?}", form);
    book(&session, &form, &data, &cache_map, &setting_map, "update").await
}

pub async fn delete_book(
    session: Session,
    form: web::Json<UpdateBookForm>,
    data: web::Data<Mutex<ClientHolder>>,
    cache_map: web::Data<Mutex<HashMap<String, Cache>>>,
    setting_map: web::Data<Mutex<HashMap<String, SystemSetting>>>,
) -> Result<HttpResponse, BibErrorResponse> {
    debug!("{:?}", form);
    book(&session, &form, &data, &cache_map, &setting_map, "delete").await
}

async fn book(
    session: &Session,
    form: &web::Json<UpdateBookForm>,
    data: &web::Data<Mutex<ClientHolder>>,
    cache_map: &web::Data<Mutex<HashMap<String, Cache>>>,
    setting_map: &web::Data<Mutex<HashMap<String, SystemSetting>>>,
    operation: &str,
) -> Result<HttpResponse, BibErrorResponse> {
    let dbname = check_operator_session(&session)?;
    let db = get_db(&data, &session).await?;

    let setting_map = setting_map.lock().unwrap();
    let setting = setting_map.get(&dbname);
    if setting.is_none() {
        return Err(BibErrorResponse::NotAuthorized);
    }
    let setting = setting.unwrap().clone();
    drop(setting_map);

    // Read the Book from DB first
    let mut book = Book::default();
    book.id = atoi(&form.book_id).map_err(|e| BibErrorResponse::InvalidArgument(e.to_string()))?;
    book = match search_item(&db, &book).await {
        Ok(mut book) => {
            if operation == "insert" {
                return Err(BibErrorResponse::ItemAlreadyExists(book.id));
            }
            book.title = form.book_title.clone();
            book.location = form.book_location.clone();
            book.category = form.book_category.clone();
            book.status = form.book_status.clone();
            book.author = form.book_author.clone();
            book.publisher = form.book_publisher.clone();
            book.published_date = form.book_published_date.clone();
            book.series = form.book_series.clone();
            book.volume = form.book_volume.clone();
            book.page = form.book_page.clone();
            book.kana = form.book_kana.clone();
            book.category_symbol = form.book_category_symbol.clone();
            book.library_symbol = form.book_library_symbol.clone();
            book.volume_symbol = form.book_volume_symbol.clone();
            book.forbidden = form.book_forbidden.clone();
            book.remark = form.book_remark.clone();
            book.register_date = form.book_register_date.clone();
            book.register_type = form.book_register_type.clone();
            book
        }
        Err(_) => {
            // Check the number of items
            book.id = 0;
            let books = match search_items(&db, &book).await {
                Ok(books) => books,
                Err(_) => vec![],
            };
            let nsize: u32 = books.len().try_into().unwrap();
            if nsize == setting.max_registered_users {
                return Err(BibErrorResponse::ExceedLimit(nsize));
            }

            Book::new(
                &form.book_id,
                &form.book_title,
                &form.book_location,
                &form.book_category,
                &form.book_status,
                &form.book_author,
                &form.book_publisher,
                &form.book_published_date,
                &form.book_series,
                &form.book_volume,
                &form.book_page,
                &form.book_kana,
                &form.book_category_symbol,
                &form.book_library_symbol,
                &form.book_volume_symbol,
                &form.book_forbidden,
                &form.book_remark,
                &form.book_isbn,
                &form.book_register_date,
                &form.book_register_type,
            )
            .unwrap()
        }
    };

    match operation {
        "insert" => {
            insert_item(&db, &book)
                .await
                .map_err(|e| BibErrorResponse::SystemError(e.to_string()))?;
        }
        "update" => {
            update_item(&db, &book)
                .await
                .map_err(|e| BibErrorResponse::SystemError(e.to_string()))?;
        }
        "delete" => {
            let cache_map = cache_map.lock().unwrap();
            let cache = cache_map.get(&dbname);
            if cache.is_none() {
                return Err(BibErrorResponse::NotAuthorized);
            }
            let cache = cache.unwrap();

            if cache.get(book.id).is_some() {
                return Err(BibErrorResponse::NotPossibleToDelete);
            }
            delete_item(&db, &book)
                .await
                .map_err(|e| BibErrorResponse::SystemError(e.to_string()))?;
        }
        _ => {
            return Err(BibErrorResponse::NotImplemented);
        }
    }

    let reply = Reply::default();
    Ok(HttpResponse::Ok().json(reply))
}
