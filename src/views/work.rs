use crate::error::*;
use crate::item::atoi;
use crate::item::RentalSetting;
use crate::item::SystemSetting;
use crate::item::{search_item, search_items, update_item_with_session};
use crate::item::{Book, BorrowedBook, User};
use crate::views::cache::*;
use crate::views::db_helper::get_db;
use crate::views::reply::Reply;
use crate::views::session::*;
use crate::views::transaction::*;
use crate::views::utils::get_nowtime;
use actix_session::Session;
use actix_web::{web, HttpResponse, Result};
use lazy_static::lazy_static;
use log::{debug, error, info};
use mongodb::{ClientSession, Database};
use serde::Deserialize;
use shared_mongodb::database::{abort_transaction, commit_transaction, start_transaction};
use shared_mongodb::{database, ClientHolder};
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::Mutex as AsyncMutex;

const BOOK_BARCODE_KETA: usize = 7;

#[derive(Deserialize, Debug)]
pub struct FormData {
    pub user_id: String,
    pub borrowed_book_id: String,
    pub returned_book_id: String,
}

lazy_static! {
    static ref GIANT_LOCK: AsyncMutex<()> = AsyncMutex::new(());
}

pub async fn process(
    session: Session,
    form: web::Form<FormData>,
    data: web::Data<Mutex<ClientHolder>>,
    cache_map: web::Data<HashMap<String, Cache>>,
    setting_map: web::Data<HashMap<String, SystemSetting>>,
    transaction_map: web::Data<HashMap<String, Transaction>>,
) -> Result<HttpResponse, BibErrorResponse> {
    debug!("{:?}", form);

    check_session(&session)?;
    let db = get_db(&data, &session).await?;
    let dbname = get_string_value(&session, "dbname")?;

    let system_setting = setting_map.get(&dbname);
    if system_setting.is_none() {
        return Err(BibErrorResponse::NotAuthorized);
    }
    let system_setting = system_setting.unwrap();

    let mut setting = RentalSetting::default();
    setting.id = 1;
    let mut setting = match search_items(&db, &setting).await {
        Ok(setting) => setting,
        Err(e) => {
            database::disconnect(&data);
            return Err(BibErrorResponse::DataNotFound(e.to_string()));
        }
    };
    if setting.len() != 1 {
        return Err(BibErrorResponse::DataDuplicated(0));
    }
    let setting = setting.pop().unwrap();

    let cache = cache_map.get(&dbname);
    if cache.is_none() {
        return Err(BibErrorResponse::NotAuthorized);
    }
    let cache = cache.unwrap();

    let transaction = transaction_map.get(&dbname);
    if transaction.is_none() {
        return Err(BibErrorResponse::NotAuthorized);
    }
    let transaction = transaction.unwrap();

    // Lock the whole process
    let _lock = GIANT_LOCK.lock().await;

    let mut user = User::default();
    if form.user_id == "" && form.borrowed_book_id == "" && form.returned_book_id != "" {
        let mut session = start_transaction(&data)
            .await
            .map_err(|e| BibErrorResponse::SystemError(e.to_string()))?;
        let ret = unborrow_book(
            &db,
            &cache,
            &transaction,
            &mut user,
            &form.returned_book_id,
            &system_setting.time_zone,
            &mut session,
        )
        .await;
        if ret.is_err() {
            match abort_transaction(&mut session).await {
                Ok(_) => {}
                Err(e) => {
                    error!("{}", e.to_string());
                }
            }
            return Err(ret.unwrap_err());
        }
        match commit_transaction(&mut session).await {
            Ok(_) => {}
            Err(e) => {
                return Err(BibErrorResponse::SystemError(e.to_string()));
            }
        }
        let (book_title, book_id) = ret.unwrap();
        let mut reply = Reply::default();
        reply.returned_book_title = book_title;
        reply.returned_book_id = book_id;
        reply.user = user;
        return Ok(HttpResponse::Ok().json(reply));
    }

    user.id = atoi(&form.user_id).map_err(|e| BibErrorResponse::InvalidArgument(e.to_string()))?;
    let mut user = match search_item(&db, &user).await {
        Ok(user) => user,
        Err(_) => {
            database::disconnect(&data);
            return Err(BibErrorResponse::UserNotFound(user.id));
        }
    };

    if form.borrowed_book_id != "" {
        // Create a DB session
        let mut session = start_transaction(&data)
            .await
            .map_err(|e| BibErrorResponse::SystemError(e.to_string()))?;

        let ret = borrow_book(
            &db,
            &cache,
            &transaction,
            &mut user,
            &form.borrowed_book_id,
            &system_setting.time_zone,
            setting.num_books,
            setting.num_days.into(),
            &mut session,
        )
        .await;
        if ret.is_err() {
            // Roll back the transaction
            match abort_transaction(&mut session).await {
                Ok(_) => {}
                Err(e) => {
                    error!("{}", e.to_string());
                }
            }
            return Err(ret.unwrap_err());
        }

        // Commit the transaction
        match commit_transaction(&mut session).await {
            Ok(_) => {}
            Err(e) => {
                return Err(BibErrorResponse::SystemError(e.to_string()));
            }
        }
    }

    if form.returned_book_id != "" {
        // Create a DB session
        let mut session = start_transaction(&data)
            .await
            .map_err(|e| BibErrorResponse::SystemError(e.to_string()))?;
        let ret = unborrow_book(
            &db,
            &cache,
            &transaction,
            &mut user,
            &form.returned_book_id,
            &system_setting.time_zone,
            &mut session,
        )
        .await;
        if ret.is_err() {
            // Roll back the transaction
            match abort_transaction(&mut session).await {
                Ok(_) => {}
                Err(e) => {
                    info!("{}", e.to_string());
                }
            }
            return Err(ret.unwrap_err());
        }

        // Commit the transaction
        match commit_transaction(&mut session).await {
            Ok(_) => {}
            Err(e) => {
                return Err(BibErrorResponse::SystemError(e.to_string()));
            }
        }
    }

    let mut reply = Reply::default();
    reply.user = user.clone();
    for book in user.borrowed_books {
        // Insert the new item at the front to sort in the order of the date
        reply.borrowed_books.insert(0, book.clone());
    }

    Ok(HttpResponse::Ok().json(reply))
}

async fn borrow_book(
    db: &Database,
    cache: &Cache,
    transaction: &Transaction,
    user: &mut User,
    book_id: &str,
    time_zone: &str,
    max_borrowing_books: u32,
    max_borrowing_days: i64,
    session: &mut ClientSession,
) -> Result<(), BibErrorResponse> {
    let num_borrowed_books: u32 = user.borrowed_books.len().try_into().unwrap();
    if num_borrowed_books >= max_borrowing_books {
        return Err(BibErrorResponse::OverBorrowingLimit);
    }

    // Check the barcode size
    if book_id.starts_with("0") && book_id.len() != BOOK_BARCODE_KETA {
        return Err(BibErrorResponse::InvalidArgument(book_id.to_owned()));
    }

    // Check if the book exists
    let mut book = Book::default();
    let book_id = atoi(book_id).map_err(|e| BibErrorResponse::InvalidArgument(e.to_string()))?;
    book.id = book_id;
    let mut books = match search_items(db, &book).await {
        Ok(books) => books,
        Err(_) => {
            return Err(BibErrorResponse::BookNotFound(book.id));
        }
    };
    if books.len() != 1 {
        return Err(BibErrorResponse::DataDuplicated(book.id));
    }

    let mut book = books.pop().unwrap();
    let borrow_info = cache.get(book.id);
    if borrow_info.is_some() {
        info!("book_id({}) is hit in the cached", book_id);
        return Err(BibErrorResponse::BookNotReturned);
    }

    // Increment the transaction counter
    let mut counter = transaction.counter.lock().unwrap();
    *counter += 1;
    let mut transaction_id = *counter % (transaction.max_counter + 1);
    if transaction_id == 0 {
        transaction_id = 1;
    }
    *counter = transaction_id;
    drop(counter);

    // Record the length of borrowed_books before removing the book
    let initial_borrowed_books_length = user.borrowed_books.len();

    let borrowed_book = BorrowedBook::new(
        book_id,
        &book.title,
        get_nowtime(time_zone),
        max_borrowing_days,
        transaction_id,
        book.char.clone(),
    );
    let return_deadline = borrowed_book.return_deadline.clone();
    user.borrowed_books.push(borrowed_book);
    user.borrowed_count += 1;

    // Update the DB (with session for rollback support)
    update_item_with_session(db, user, session)
        .await
        .map_err(|e| BibErrorResponse::SystemError(e.to_string()))?;

    // Check if the length of borrowed_books has changed unexpectedly
    if user.borrowed_books.len() != initial_borrowed_books_length + 1 {
        info!("Unexpected change detected in borrowed_books, rolling back");
        return Err(BibErrorResponse::ConcurrencyError(
            "Unexpected change in borrowed_books".to_string(),
        ));
    }

    book.borrowed_count += 1;
    update_item_with_session(db, &book, session)
        .await
        .map_err(|e| BibErrorResponse::SystemError(e.to_string()))?;

    Transaction::borrow_with_session(db, transaction_id, user, &book, time_zone, session)
        .await
        .map_err(|e| BibErrorResponse::SystemError(e.to_string()))?;

    // Update the cache
    cache.borrow(book.id, user.id, return_deadline);

    Ok(())
}

async fn unborrow_book(
    db: &Database,
    cache: &Cache,
    _transaction: &Transaction,
    user: &mut User,
    book_id: &str,
    time_zone: &str,
    session: &mut ClientSession,
) -> Result<(String, u32), BibErrorResponse> {
    // Check the barcode size
    if book_id.starts_with("0") && book_id.len() != BOOK_BARCODE_KETA {
        return Err(BibErrorResponse::InvalidArgument(book_id.to_owned()));
    }

    // Check if the book exists
    let mut book = Book::default();
    let book_id = atoi(book_id).map_err(|e| BibErrorResponse::InvalidArgument(e.to_string()))?;
    book.id = book_id;
    let mut books = match search_items(db, &book).await {
        Ok(books) => books,
        Err(_) => {
            return Err(BibErrorResponse::BookNotFound(book.id));
        }
    };
    if books.len() != 1 {
        return Err(BibErrorResponse::DataDuplicated(book.id));
    }
    book = books.pop().unwrap();

    // Read the user data and check if the book is borrowed
    if user.id == 0 {
        let borrow_info = cache.get(book.id);
        if borrow_info.is_none() {
            info!("book_id({}) is NOT hit in the cached", book_id);
            return Err(BibErrorResponse::BookNotBorrowed);
        }
        user.id = borrow_info.unwrap().owner_id;
        *user = match search_item(db, user).await {
            Ok(user) => user,
            Err(_) => {
                return Err(BibErrorResponse::UserNotFound(user.id));
            }
        };
    }

    // Record the length of borrowed_books before removing the book
    let initial_borrowed_books_length = user.borrowed_books.len();

    let mut transaction_id: u32 = 0;
    let mut done: bool = false;
    let mut borrowed_date: String = String::new();
    for (pos, borrowed_book) in user.borrowed_books.iter().enumerate() {
        if borrowed_book.book_id == book_id {
            transaction_id = borrowed_book.transaction_id;
            borrowed_date = borrowed_book.borrowed_date.clone();
            user.borrowed_books.remove(pos);
            done = true;
            break;
        }
    }
    if !done {
        info!("book_id({}) is not hit in the User DB", book_id);
        return Err(BibErrorResponse::BookNotBorrowed);
    }

    // Update the DB (with session for rollback support)
    update_item_with_session(db, user, session)
        .await
        .map_err(|e| BibErrorResponse::SystemError(e.to_string()))?;

    // Check if the length of borrowed_books has changed unexpectedly
    if user.borrowed_books.len() != initial_borrowed_books_length - 1 {
        info!("Unexpected change detected in borrowed_books, rolling back");
        return Err(BibErrorResponse::ConcurrencyError(
            "Unexpected change in borrowed_books".to_string(),
        ));
    }

    Transaction::unborrow_with_session(
        db,
        transaction_id,
        user,
        &book,
        borrowed_date,
        time_zone,
        session,
    )
    .await
    .map_err(|e| BibErrorResponse::SystemError(e.to_string()))?;

    // Update the cache
    cache.unborrow(book.id);

    Ok((book.title, book.id))
}
