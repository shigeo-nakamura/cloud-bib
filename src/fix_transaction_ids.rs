use bson::doc;
use futures::stream::TryStreamExt;
use log::info;
use mongodb::options::{
    ClientOptions, FindOneAndUpdateOptions, FindOptions, ReturnDocument, Tls, TlsOptions,
};
use mongodb::Database;
use serde::{Deserialize, Serialize};
use std::env;

// Rotated IDs are currently in the 1001-1289 range (already +1000 from first fix).
// They were assigned on 3 different dates due to Heroku restarts:
//   2/14 — first cycle
//   2/28 — second cycle (same IDs reused)
//   3/7  — third cycle (same IDs reused again)
// Fix: shift 2/28 by +1000 (-> 2000 range), shift 3/7 by +2000 (-> 3000 range).
// 2/14 stays as-is in the 1000 range.

const RANGE_LO: u32 = 1001;
const RANGE_HI: u32 = 1500;

const DATE_0228: &str = "2026/02/28";
const DATE_0307: &str = "2026/03/07";

const OFFSET_0228: u32 = 1000;
const OFFSET_0307: u32 = 2000;

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

    // Step 1: Backup users2 -> users2-backup
    backup_users2(&db).await;

    // Step 2: Fix transaction_ids by shifting per borrowed_date
    fix_transaction_ids(&db).await;

    info!("done");
}

async fn backup_users2(db: &Database) {
    info!("=== Backing up users2 to users2-backup ===");

    let source = db.collection::<bson::Document>("users2");
    let backup = db.collection::<bson::Document>("users2-backup");

    // Drop existing backup if any
    match backup.drop(None).await {
        Ok(_) => info!("Dropped existing users2-backup"),
        Err(e) => info!("No existing backup to drop: {:?}", e),
    }

    let find_options = FindOptions::builder().build();
    let mut cursor = source.find(doc! {}, find_options).await.unwrap();

    let mut count = 0;
    while let Some(document) = cursor.try_next().await.unwrap() {
        backup.insert_one(document, None).await.unwrap();
        count += 1;
    }
    info!("Backed up {} documents to users2-backup", count);
}

async fn fix_transaction_ids(db: &Database) {
    info!("=== Fixing transaction_ids in users2 ===");
    info!(
        "Target range: {} - {}, shift 2/28 by +{}, shift 3/7 by +{}",
        RANGE_LO, RANGE_HI, OFFSET_0228, OFFSET_0307
    );

    let collection = db.collection::<User>("users2");
    let find_options = FindOptions::builder().sort(doc! { "id": 1 }).build();
    let mut cursor = collection
        .find(doc! { "id": { "$gt": 0 } }, find_options)
        .await
        .unwrap();

    let mut updated_count = 0;
    while let Some(user) = cursor.try_next().await.unwrap() {
        let mut new_books = user.borrowed_books.clone();
        let mut modified = false;

        for book in new_books.iter_mut() {
            if book.transaction_id < RANGE_LO || book.transaction_id > RANGE_HI {
                continue;
            }

            let offset = if book.borrowed_date.starts_with(DATE_0307) {
                OFFSET_0307
            } else if book.borrowed_date.starts_with(DATE_0228) {
                OFFSET_0228
            } else {
                // 2/14 or other dates: keep as-is in the 1000 range
                0
            };

            if offset > 0 {
                info!(
                    "User {} ({}): book_id={}, borrowed={}, transaction_id {} -> {}",
                    user.id,
                    user.name,
                    book.book_id,
                    book.borrowed_date,
                    book.transaction_id,
                    book.transaction_id + offset
                );
                book.transaction_id += offset;
                modified = true;
            }
        }

        if modified {
            let updated_user = User {
                borrowed_books: new_books,
                ..user.clone()
            };
            let query = doc! { "id": user.id };
            let update_doc = bson::to_bson(&updated_user).unwrap();
            let update = doc! { "$set": update_doc };
            let options = FindOneAndUpdateOptions::builder()
                .upsert(false)
                .return_document(ReturnDocument::After)
                .build();

            match collection
                .find_one_and_update(query, update, options)
                .await
            {
                Ok(_) => {
                    updated_count += 1;
                    info!("Updated user {}", user.id);
                }
                Err(e) => {
                    panic!("Failed to update user {}: {:?}", user.id, e);
                }
            }
        }
    }
    info!("Updated {} users", updated_count);
}
