use crate::item::*;
use crate::item::{Book, User};
use crate::views::utils::get_nowtime;
use futures::stream::TryStreamExt;
use log::{debug, info};
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use mongodb::{ClientSession, Database};
use std::error;
use std::sync::Mutex;

pub struct Transaction {
    pub max_counter: u32,
    pub counter: Mutex<u32>,
}

impl Transaction {
    pub fn new(max_counter: u32, counter: u32) -> Self {
        Transaction {
            max_counter: max_counter,
            counter: Mutex::new(counter),
        }
    }

    pub async fn search(db: &Database, item: &TransactionItem) -> Vec<TransactionItem> {
        debug!("{:?}", item);
        let items = match search_items(db, item).await {
            Ok(items) => items,
            Err(e) => {
                info!("{:?}", e);
                vec![]
            }
        };
        items
    }

    /// Find the last counter by looking at the most recently borrowed transaction.
    /// Sort by borrowed_date descending to find the latest transaction regardless of ID rotation.
    pub async fn find_latest_counter(db: &Database) -> u32 {
        let collection = db.collection::<TransactionItem>("transactions");
        let find_options = FindOptions::builder()
            .sort(doc! { "borrowed_date": -1 })
            .limit(1)
            .build();
        let query = doc! { "id": { "$gt": 0 } };
        match collection.find(query, find_options).await {
            Ok(mut cursor) => match cursor.try_next().await {
                Ok(Some(item)) => {
                    info!(
                        "find_latest_counter: id={}, borrowed_date={}",
                        item.id, item.borrowed_date
                    );
                    item.id
                }
                _ => 0,
            },
            Err(e) => {
                info!("find_latest_counter error: {:?}", e);
                0
            }
        }
    }

    pub async fn borrow_with_session(
        db: &Database,
        counter: u32,
        user: &User,
        book: &Book,
        time_zone: &str,
        session: &mut ClientSession,
    ) -> Result<(), Box<dyn error::Error>> {
        let dt = get_nowtime(time_zone);
        let item = TransactionItem {
            id: counter,
            user_id: user.id,
            user_name: user.name.clone(),
            book_id: book.id,
            book_title: book.title.clone(),
            borrowed_date: format!("{}", dt.format("%Y/%m/%d %H:%M")),
            returned_date: "".to_string(),
        };
        debug!("borrow_with_session: {:?}, counter={}", item, counter);
        update_item_with_session(db, &item, session).await
    }

    pub async fn unborrow_with_session(
        db: &Database,
        counter: u32,
        user: &User,
        book: &Book,
        borrowed_date: String,
        time_zone: &str,
        session: &mut ClientSession,
    ) -> Result<(), Box<dyn error::Error>> {
        let dt = get_nowtime(time_zone);
        let item = TransactionItem {
            id: counter,
            user_id: user.id,
            user_name: user.name.clone(),
            book_id: book.id,
            book_title: book.title.clone(),
            borrowed_date: borrowed_date,
            returned_date: format!("{}", dt.format("%Y/%m/%d %H:%M")),
        };
        debug!("unborrow_with_session: {:?}, counter={}", item, counter);
        update_item_with_session(db, &item, session).await
    }
}
