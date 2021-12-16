use crate::db_client::*;
use crate::error::BibErrorResponse;
use crate::item::atoi;
use crate::item::search_items;
use crate::item::User;
use crate::views::reply::Reply;
use crate::views::session::check_session;
use actix_session::Session;
use actix_web::{web, HttpResponse, Result};
use log::debug;
use serde::Deserialize;
use std::sync::Mutex;

#[derive(Deserialize, Debug)]
pub struct FormData {
    pub id: String,
    pub name: String,
    pub kana: String,
    pub category: String,
}

pub async fn search_user(
    session: Session,
    form: web::Query<FormData>,
    data: web::Data<Mutex<DbClient>>,
) -> Result<HttpResponse, BibErrorResponse> {
    debug!("{:?}", form);
    check_session(&session)?;

    let mut user = User::default();
    if form.id == "" {
        user.id = 0;
    } else {
        user.id = atoi(&form.id).map_err(|e| BibErrorResponse::InvalidArgument(e.to_string()))?;
    }
    user.name = form.name.clone();
    user.kana = form.kana.clone();
    user.category = form.category.clone();
    get_user_list(data, &user).await
}

async fn get_user_list(
    data: web::Data<Mutex<DbClient>>,
    user: &User,
) -> Result<HttpResponse, BibErrorResponse> {
    let db = get_db(&data).await?;

    let mut users = match search_items(&db, user).await {
        Ok(users) => users,
        Err(e) => {
            return Err(BibErrorResponse::DataNotFound(e.to_string()));
        }
    };

    let mut reply = Reply::default();
    reply.user_list.append(&mut users);

    Ok(HttpResponse::Ok().json(reply))
}
