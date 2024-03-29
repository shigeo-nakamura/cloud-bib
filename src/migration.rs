use crate::item::create_unique_index;
use crate::item::TransactionItem;
use crate::item::*;
use crate::views::cache::Cache;
use crate::views::transaction::*;
use actix_session::CookieSession;
use actix_web::{web, App, HttpServer};
use log::{debug, info};
use mongodb::options::{ClientOptions, Tls, TlsOptions};
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use shared_mongodb::{database, ClientHolder};
use std::env;
use std::sync::Mutex;

mod error;
mod item;
mod views;

#[actix_rt::main]
async fn main() {
    env_logger::init();

    let client_uri =
        env::var("BIB_MONGODB_URI").expect("You must set the BIB_MONGODB_URI environment var!");
    let mut client_options = match ClientOptions::parse(client_uri).await {
        Ok(client_options) => client_options,
        Err(e) => {
            panic!("{:?}", e);
        }
    };
    let tls_options = TlsOptions::builder().build();
    client_options.tls = Some(Tls::Enabled(tls_options));

    let client_holder = web::Data::new(Mutex::new(ClientHolder::new(client_options)));
    let db_name =
        env::var("BIB_DB_MEMBER_NAME").expect("You must set the DATABSE_NAME environment var!");
    let db = database::get(&client_holder.clone(), &db_name)
        .await
        .unwrap();

    let mut book = Book::default();
    book.id = 0;
    let books = match search_items(&db, &book).await {
        Ok(books) => books,
        Err(_) => {
            panic!("failed to search books");
        }
    };

    for mut book in books {
        let b = book.char.as_bytes();
        let n = b.len();
        let mut b2: Vec<u8> = vec![];
        let mut found = false;
        for i in 0..n {
            //info!("{}", b[i]);
            if b[i] == 0 {
                found = true;
            } else {
                b2.push(b[i]);
            }
        }
        if found {
            info!(
                "id = {}, char = {}/{:?}",
                book.id,
                book.char,
                book.char.as_bytes()
            );
            let new_char = std::str::from_utf8(&b2).unwrap();
            info!("{}", new_char);
            info!(
                "id = {}, char = {}/{:?}",
                book.id,
                new_char,
                new_char.as_bytes()
            );

            book.char = new_char.to_string();
            info!("new book = {:?}", book);
            info!("----------------------------------------------");

            match update_item(&db, &book).await {
                Ok(_) => {}
                Err(_) => {
                    panic!("update failed");
                }
            }
        }
    }

    info!("done");
}
