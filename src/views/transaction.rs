use crate::item::*;
use crate::item::{Book, User};
use crate::views::utils::get_nowtime;
use log::{debug, info};
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
