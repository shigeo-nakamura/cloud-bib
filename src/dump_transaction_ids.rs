use bson::doc;
use futures::stream::TryStreamExt;
use mongodb::options::{ClientOptions, FindOptions, Tls, TlsOptions};
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
    pub borrowed_books: Vec<BorrowedBook>,
}

#[tokio::main]
async fn main() {
    let client_uri =
        env::var("BIB_MONGODB_URI").expect("You must set the BIB_MONGODB_URI environment var!");
    let mut client_options = ClientOptions::parse(client_uri).await.unwrap();
    let tls_options = TlsOptions::builder().build();
    client_options.tls = Some(Tls::Enabled(tls_options));

    let client = mongodb::Client::with_options(client_options).unwrap();
    let db_name = env::var("BIB_DB_MEMBER_NAME")
        .expect("You must set the BIB_DB_MEMBER_NAME environment var!");
    let db = client.database(&db_name);

    let collection = db.collection::<User>("users2");
    let find_options = FindOptions::builder().sort(doc! { "id": 1 }).build();
    let mut cursor = collection
        .find(doc! { "id": { "$gt": 0 } }, find_options)
        .await
        .unwrap();

    // transaction_id -> list of (user_id, user_name, book_id, borrowed_date)
    let mut all_ids: HashMap<u32, Vec<(u32, String, u32, String)>> = HashMap::new();
    let mut all_tids: Vec<u32> = Vec::new();

    while let Some(user) = cursor.try_next().await.unwrap() {
        for book in &user.borrowed_books {
            all_ids
                .entry(book.transaction_id)
                .or_default()
                .push((user.id, user.name.clone(), book.book_id, book.borrowed_date.clone()));
            all_tids.push(book.transaction_id);
        }
    }

    all_tids.sort();

    println!("=== All transaction_ids in borrowed_books (sorted) ===");
    println!("Total: {} entries, {} unique IDs", all_tids.len(), all_ids.len());
    println!();

    // Show distribution by range
    let ranges = [
        (0, 100),
        (100, 500),
        (500, 1000),
        (1000, 1500),
        (1500, 2000),
        (2000, 5000),
        (5000, 10000),
        (10000, 49000),
        (49000, 50001),
    ];
    println!("=== Distribution by range ===");
    for (lo, hi) in &ranges {
        let count = all_tids.iter().filter(|&&t| t >= *lo && t < *hi).count();
        if count > 0 {
            println!("  [{:>5} - {:>5}): {} entries", lo, hi, count);
        }
    }

    println!();
    println!("=== Duplicated transaction_ids ===");
    let mut dup_ids: Vec<u32> = all_ids
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, _)| *k)
        .collect();
    dup_ids.sort();

    for tid in &dup_ids {
        let entries = &all_ids[tid];
        println!("transaction_id={} ({} uses):", tid, entries.len());
        for (user_id, user_name, book_id, borrowed_date) in entries {
            println!(
                "  user={} ({}), book_id={}, borrowed={}",
                user_id, user_name, book_id, borrowed_date
            );
        }
    }

    println!();
    println!("=== Min/Max ===");
    if let (Some(min), Some(max)) = (all_tids.first(), all_tids.last()) {
        println!("Min transaction_id: {}", min);
        println!("Max transaction_id: {}", max);
    }
}
