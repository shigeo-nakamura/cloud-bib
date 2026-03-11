use bson::doc;
use futures::stream::TryStreamExt;
use log::info;
use mongodb::options::{ClientOptions, FindOptions, Tls, TlsOptions};
use mongodb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct BorrowedBook {
    pub book_id: u32,
    pub book_title: String,
    pub borrowed_date: String,
    pub return_deadline: String,
    pub transaction_id: u32,
    pub char: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct User {
    pub id: u32,
    pub name: String,
    pub kana: String,
    pub category: String,
    pub remark: String,
    pub register_date: String,
    pub borrowed_count: u32,
    pub reserved: String,
    pub borrowed_books: Vec<BorrowedBook>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct TransactionItem {
    pub id: u32,
    pub user_id: u32,
    pub user_name: String,
    pub book_id: u32,
    pub book_title: String,
    pub borrowed_date: String,
    pub returned_date: String,
}

#[tokio::main]
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

    let client = mongodb::Client::with_options(client_options).unwrap();
    let db_name = env::var("BIB_DB_MEMBER_NAME")
        .expect("You must set the BIB_DB_MEMBER_NAME environment var!");
    let db = client.database(&db_name);

    check_duplicates(&db).await;
}

async fn check_duplicates(db: &Database) {
    // Collect all transaction_ids from borrowed_books in users2
    info!("=== Collecting transaction_ids from users2.borrowed_books ===");

    let user_collection = db.collection::<User>("users2");
    let find_options = FindOptions::builder().sort(doc! { "id": 1 }).build();
    let mut cursor = user_collection
        .find(doc! { "id": { "$gt": 0 } }, find_options)
        .await
        .unwrap();

    // transaction_id -> (user_id, user_name, book_id, book_title)
    let mut borrowed_ids: HashMap<u32, Vec<(u32, String, u32, String)>> = HashMap::new();

    while let Some(user) = cursor.try_next().await.unwrap() {
        for book in &user.borrowed_books {
            borrowed_ids
                .entry(book.transaction_id)
                .or_default()
                .push((user.id, user.name.clone(), book.book_id, book.book_title.clone()));
        }
    }

    info!(
        "Found {} unique transaction_ids in borrowed_books",
        borrowed_ids.len()
    );

    // Check for duplicates within borrowed_books themselves
    let mut dup_count = 0;
    for (tid, entries) in &borrowed_ids {
        if entries.len() > 1 {
            println!(
                "[DUPLICATE in borrowed_books] transaction_id={} used {} times:",
                tid,
                entries.len()
            );
            for (user_id, user_name, book_id, book_title) in entries {
                println!(
                    "  user_id={}, name={}, book_id={}, title={}",
                    user_id, user_name, book_id, book_title
                );
            }
            dup_count += 1;
        }
    }

    // Check for conflicts with transactions collection
    info!("=== Checking conflicts with transactions collection ===");

    let tx_collection = db.collection::<TransactionItem>("transactions");
    let mut conflict_count = 0;

    for (tid, entries) in &borrowed_ids {
        let query = doc! { "id": *tid };
        let find_options = FindOptions::builder().build();
        let mut cursor = tx_collection.find(query, find_options).await.unwrap();

        while let Some(tx) = cursor.try_next().await.unwrap() {
            // Check if the transaction record belongs to a different user/book
            for (user_id, user_name, book_id, book_title) in entries {
                if tx.user_id != *user_id || tx.book_id != *book_id {
                    println!(
                        "[CONFLICT] transaction_id={}: borrowed_books has user={}({})/book={}({}), but transactions has user={}({})/book={}({}), borrowed_date={}, returned_date={}",
                        tid,
                        user_id, user_name, book_id, book_title,
                        tx.user_id, tx.user_name, tx.book_id, tx.book_title,
                        tx.borrowed_date, tx.returned_date
                    );
                    conflict_count += 1;
                }
            }
        }
    }

    println!();
    println!("=== Summary ===");
    println!(
        "Duplicate transaction_ids within borrowed_books: {}",
        dup_count
    );
    println!(
        "Conflicts between borrowed_books and transactions: {}",
        conflict_count
    );

    if dup_count == 0 && conflict_count == 0 {
        println!("OK: No duplicates or conflicts found.");
    }
}
