use crate::error::BibErrorResponse;
use crate::item::search_items;
use crate::item::SystemSetting;
use crate::item::User;
use crate::views::content_loader::read_file;
use crate::views::db_helper::get_db;
use crate::views::db_helper::get_db_with_name;
use crate::views::reply::Reply;
use crate::views::session::*;
use actix_session::*;
use actix_web::{web, HttpResponse, Result};
use serde::Deserialize;
use shared_mongodb::{database, ClientHolder};
use std::env;
use std::sync::Mutex;

use lazy_static::lazy_static;

lazy_static! {
    static ref DB_MEMBER_NAME: String =
        env::var("BIB_DB_MEMBER_NAME").unwrap_or("unknown".to_string());
}

#[derive(Deserialize, Debug)]
pub struct FormData1 {
    pub user_id: String,
    pub member_password: String,
}

pub async fn load() -> HttpResponse {
    let html_data = read_file("src/html/member.html").unwrap();
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html_data)
}

pub async fn login(
    session: Session,
    form: web::Form<FormData1>,
    data: web::Data<Mutex<ClientHolder>>,
) -> Result<HttpResponse, BibErrorResponse> {
    let db = get_db_with_name(&data, &DB_MEMBER_NAME).await?;

    let user = User::new(&form.user_id, "", "", "", "", "")
        .map_err(|e| BibErrorResponse::InvalidArgument(e.to_string()))?;
    let mut users = match search_items(&db, &user).await {
        Ok(users) => users,
        Err(_) => {
            database::disconnect(&data);
            return Err(BibErrorResponse::UserNotFound(user.id));
        }
    };
    if users.len() != 1 {
        return Err(BibErrorResponse::DataDuplicated(0));
    }
    let user = users.pop().unwrap();

    let mut setting = SystemSetting::default();
    setting.id = 1;
    let mut setting = match search_items(&db, &setting).await {
        Ok(setting) => setting,
        Err(e) => {
            database::disconnect(&data);
            return Err(BibErrorResponse::DataNotFound(e.to_string()));
        }
    };

    let mut passed = false;
    if setting.len() == 1 {
        let setting = setting.pop().unwrap();
        if setting.member_password == form.member_password {
            passed = true;
            check_or_create_member_session(&session, user.id, &setting.dbname)?;
        }
    }
    if passed == false {
        return Err(BibErrorResponse::LoginFailed);
    }

    let reply = Reply::default();
    Ok(HttpResponse::Ok().json(reply))
}

pub async fn load_home(session: Session) -> HttpResponse {
    let user_id = check_member_session(&session).unwrap_or(0);

    let html_data = read_file("src/html/member_home.html")
        .unwrap()
        .replace("{{USER_ID}}", &user_id.to_string());
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html_data)
}

pub async fn load_news(_: Session) -> HttpResponse {
    let html_data = read_file("src/html/member_news.html").unwrap();
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html_data)
}

pub async fn load_search(_session: Session) -> HttpResponse {
    let html_data = read_file("src/html/member_search.html").unwrap();
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html_data)
}

pub async fn borrowed_books(
    session: Session,
    data: web::Data<Mutex<ClientHolder>>,
) -> Result<HttpResponse, BibErrorResponse> {
    let user_id = check_member_session(&session)?;
    let mut reply = Reply::default();

    let db = get_db(&data, &session).await?;

    let mut user = User::default();
    user.id = user_id;
    let mut users = match search_items(&db, &user).await {
        Ok(users) => users,
        Err(_) => {
            database::disconnect(&data);
            return Err(BibErrorResponse::UserNotFound(user.id));
        }
    };

    if users.len() != 1 {
        return Err(BibErrorResponse::DataDuplicated(user.id));
    }
    let user = users.pop().unwrap();

    for book in user.borrowed_books {
        // Insert the new item at the front to sort in the order of the date
        reply.borrowed_books.insert(0, book.clone());
    }

    Ok(HttpResponse::Ok().json(reply))
}
