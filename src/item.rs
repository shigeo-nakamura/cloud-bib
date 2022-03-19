use crate::db_client::*;
use async_trait::async_trait;
use chrono::{Duration, TimeZone, Utc};
use chrono_tz::Europe::Berlin;
use mongodb::bson::doc;
use mongodb::options::{DropCollectionOptions, IndexOptions};
use mongodb::{Collection, IndexModel};
use serde::{Deserialize, Serialize};
use std::error;

#[async_trait]
pub trait Database {
    async fn insert(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>>;
    async fn update(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>>;
    async fn delete(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>>;
    async fn delete_all(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>>;

    async fn search(&self, db: &DbInstance) -> Result<Vec<Self>, Box<dyn error::Error>>
    where
        Self: std::marker::Sized;

    fn get_collection_name(&self) -> &str;

    fn get_collection(&self, db: &DbInstance) -> Collection<Self>
    where
        Self: std::marker::Sized,
    {
        db.instance.collection::<Self>(self.get_collection_name())
    }

    async fn create_unique_index(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>>
    where
        Self: std::marker::Sized,
        Self: std::marker::Send,
    {
        let options = IndexOptions::builder().unique(true).build();
        let model = IndexModel::builder()
            .keys(doc! {"id": 1})
            .options(options)
            .build();
        let collection = self.get_collection(db);
        collection.create_index(model, None).await?;
        Ok(())
    }
}

pub async fn insert_item<T: Database>(
    db: &DbInstance,
    item: &T,
) -> Result<(), Box<dyn error::Error>> {
    item.insert(db).await
}

pub async fn update_item<T: Database>(
    db: &DbInstance,
    item: &T,
) -> Result<(), Box<dyn error::Error>> {
    item.update(db).await
}

pub async fn delete_item<T: Database>(
    db: &DbInstance,
    item: &T,
) -> Result<(), Box<dyn error::Error>> {
    item.delete(db).await
}

pub async fn delete_item_all<T: Database>(
    db: &DbInstance,
    item: &T,
) -> Result<(), Box<dyn error::Error>> {
    item.delete_all(db).await
}

pub async fn search_items<T: Database>(
    db: &DbInstance,
    item: &T,
) -> Result<Vec<T>, Box<dyn error::Error>> {
    item.search(db).await
}

pub async fn create_unique_index(db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
    let item = User::default();
    item.create_unique_index(db).await?;
    let item = Book::default();
    item.create_unique_index(db).await?;
    Ok(())
}

pub fn atoi(a: &str) -> Result<u32, Box<dyn error::Error>> {
    let i: u32 = a.to_string().parse()?;
    Ok(i)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
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
pub struct BorrowedBook {
    pub book_id: u32,
    pub book_title: String,
    pub borrowed_date: String,
    pub return_deadline: String,
    pub transaction_id: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Book {
    pub id: u32,
    pub title: String,
    pub kana: String,
    pub series: String,
    pub author: String,
    pub publisher: String,
    pub char: String,
    pub remark: String,
    pub recommendation: String,
    pub register_date: String,
    pub register_type: String,
    pub status: String,

    pub borrowed_count: u32,
    pub reserved: String,
    pub owner_id: Option<u32>,
    pub return_deadline: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RentalSetting {
    pub id: u32,
    pub num_books: u32,
    pub num_days: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemSetting {
    pub id: u32,
    pub uname: String,
    pub password: String,
    pub member_password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionItem {
    pub id: u32,
    pub user_id: u32,
    pub user_name: String,
    pub book_id: u32,
    pub book_title: String,
    pub borrowed_date: String,
    pub returned_date: String,
}

impl User {
    pub fn default() -> Self {
        let utc = Utc::now().naive_utc();
        let dt = Berlin.from_utc_datetime(&utc);
        Self {
            id: 0,
            name: String::new(),
            kana: String::new(),
            category: String::new(),
            remark: String::new(),
            register_date: format!("{}", dt.format("%Y/%m/%d")),
            borrowed_count: 0,
            reserved: String::new(),
            borrowed_books: vec![],
        }
    }

    pub fn new(
        id: &str,
        name: &str,
        kana: &str,
        category: &str,
        remark: &str,
        register_date: &str,
    ) -> Result<Self, Box<dyn error::Error>> {
        let r = Self {
            id: atoi(id)?,
            name: name.to_string(),
            kana: kana.to_string(),
            category: category.to_string(),
            remark: remark.to_string(),
            register_date: register_date.to_string(),
            borrowed_count: 0,
            reserved: String::new(),
            borrowed_books: vec![],
        };
        Ok(r)
    }
}

impl BorrowedBook {
    pub fn new(id: u32, title: &str, borrowing_days: i64, transaction_id: u32) -> Self {
        let utc = Utc::now().naive_utc();
        let dt = Berlin.from_utc_datetime(&utc);
        let deadline = dt + Duration::days(borrowing_days);
        Self {
            book_id: id,
            book_title: title.to_string(),
            borrowed_date: format!("{}", dt.format("%Y/%m/%d %H:%M")),
            return_deadline: format!("{}", deadline.format("%Y/%m/%d %H:%M")),
            transaction_id: transaction_id,
        }
    }
}

impl Book {
    pub fn default() -> Self {
        let utc = Utc::now().naive_utc();
        let dt = Berlin.from_utc_datetime(&utc);
        Self {
            id: 0,
            title: String::new(),
            kana: String::new(),
            series: String::new(),
            author: String::new(),
            publisher: String::new(),
            char: String::new(),
            remark: String::new(),
            recommendation: String::new(),
            register_date: format!("{}", dt.format("%Y/%m/%d")),
            register_type: String::new(),
            status: String::new(),
            borrowed_count: 0,
            reserved: String::new(),
            owner_id: None,
            return_deadline: None,
        }
    }

    pub fn new(
        id: &str,
        title: &str,
        kana: &str,
        series: &str,
        author: &str,
        publisher: &str,
        char: &str,
        remark: &str,
        recommendation: &str,
        register_date: &str,
        register_type: &str,
        status: &str,
    ) -> Result<Self, Box<dyn error::Error>> {
        let r = Self {
            id: atoi(id)?,
            title: title.to_string(),
            kana: kana.to_string(),
            series: series.to_string(),
            author: author.to_string(),
            publisher: publisher.to_string(),
            char: char.to_string(),
            remark: remark.to_string(),
            recommendation: recommendation.to_string(),
            register_date: register_date.to_string(),
            register_type: register_type.to_string(),
            status: status.to_string(),
            borrowed_count: 0,
            reserved: String::new(),
            owner_id: None,
            return_deadline: None,
        };
        Ok(r)
    }
}

impl RentalSetting {
    pub fn default() -> Self {
        Self {
            id: 0,
            num_books: 0,
            num_days: 0,
        }
    }

    pub fn new(num_books: &str, num_days: &str) -> Result<Self, Box<dyn error::Error>> {
        let r = Self {
            id: 0,
            num_books: atoi(num_books)?,
            num_days: atoi(num_days)?,
        };
        Ok(r)
    }
}

impl SystemSetting {
    pub fn default() -> Self {
        Self {
            id: 0,
            uname: String::new(),
            password: String::new(),
            member_password: String::new(),
        }
    }

    pub fn new(password: &str, member_password: &str) -> Result<Self, Box<dyn error::Error>> {
        let r = Self {
            id: 0,
            uname: String::new(),
            password: password.to_string(),
            member_password: member_password.to_string(),
        };
        Ok(r)
    }
}

impl TransactionItem {
    pub fn default() -> Self {
        Self {
            id: 0,
            user_id: 0,
            user_name: String::new(),
            book_id: 0,
            book_title: String::new(),
            borrowed_date: String::new(),
            returned_date: String::new(),
        }
    }
    pub fn new(user_id: u32, user_name: &str, book_id: u32, book_title: &str) -> Self {
        Self {
            id: 0,
            user_id: user_id,
            user_name: user_name.to_string(),
            book_id: book_id,
            book_title: book_title.to_string(),
            borrowed_date: String::new(),
            returned_date: String::new(),
        }
    }
}

#[async_trait]
impl Database for User {
    async fn insert(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        let collection = self.get_collection(db);
        collection.insert_one(self, None).await?;
        Ok(())
    }

    async fn update(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        let query = doc! { "id" : self.id };
        let update = bson::to_bson(self).unwrap();
        let update = doc! { "$set" : update };
        let collection = self.get_collection(db);
        collection.update(query, update, true).await
    }

    async fn delete(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        let query = doc! { "id" : self.id };
        let collection = self.get_collection(db);
        collection.delete(query).await
    }

    async fn delete_all(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        panic!("Not implemented")
    }

    async fn search(&self, db: &DbInstance) -> Result<Vec<Self>, Box<dyn error::Error>> {
        let mut query = doc! { "id": { "$gt": 0 }};

        if self.id != 0 {
            query = doc! { "id": self.id };
        } else if self.name != "" {
            query = doc! { "name": {"$regex": &self.name} };
        } else if self.kana != "" {
            query = doc! { "kana": {"$regex": &self.kana} };
        } else if self.category != "" {
            query = doc! { "category": {"$regex": &self.category} };
        }

        let collection = self.get_collection(db);
        collection.search(query).await
    }

    fn get_collection_name(&self) -> &str {
        "users"
    }
}

#[async_trait]
impl Database for Book {
    async fn insert(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        let collection = self.get_collection(db);
        collection.insert_one(self, None).await?;
        Ok(())
    }

    async fn update(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        let query = doc! { "id" : self.id };
        let update = bson::to_bson(self).unwrap();
        let update = doc! { "$set" : update };
        let collection = self.get_collection(db);
        collection.update(query, update, true).await
    }

    async fn delete(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        let query = doc! { "id" : self.id };
        let collection = self.get_collection(db);
        collection.delete(query).await
    }

    async fn delete_all(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        panic!("Not implemented")
    }

    async fn search(&self, db: &DbInstance) -> Result<Vec<Self>, Box<dyn error::Error>> {
        let mut query = doc! { "id": { "$gt": 0 }};

        if self.id != 0 {
            query = doc! { "id": self.id };
        } else if self.title != "" {
            query = doc! { "title": {"$regex": &self.title} };
        } else if self.kana != "" {
            query = doc! { "kana": {"$regex": &self.kana} };
        }

        let collection = self.get_collection(db);
        collection.search(query).await
    }

    fn get_collection_name(&self) -> &str {
        "books"
    }
}

#[async_trait]
impl Database for RentalSetting {
    async fn insert(&self, _db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        panic!("Not implemented")
    }

    async fn update(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        let query = doc! { "id": self.id };
        let update = bson::to_bson(self).unwrap();
        let update = doc! { "$set" : update };
        let collection = self.get_collection(db);
        collection.update(query, update, false).await
    }

    async fn delete(&self, _db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        panic!("Not implemented")
    }

    async fn delete_all(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        panic!("Not implemented")
    }

    async fn search(&self, db: &DbInstance) -> Result<Vec<Self>, Box<dyn error::Error>> {
        let query = doc! { "$or" : [{"id": self.id}] };
        let collection = self.get_collection(db);
        collection.search(query).await
    }

    fn get_collection_name(&self) -> &str {
        "rental-setting"
    }
}

#[async_trait]
impl Database for SystemSetting {
    async fn insert(&self, _db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        panic!("Not implemented")
    }

    async fn update(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        let query = doc! { "id": self.id };
        let mut update = doc![];
        if self.password != "" && self.member_password != "" {
            update = doc! { "password" : self.password.clone(), "member_password": self.member_password.clone() };
        } else if self.password != "" {
            update = doc! { "password" : self.password.clone() };
        } else {
            update = doc! { "member_password" : self.member_password.clone() };
        }

        let update = doc! { "$set" : update };
        let collection = self.get_collection(db);
        collection.update(query, update, false).await
    }

    async fn delete(&self, _db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        panic!("Not implemented")
    }

    async fn delete_all(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        panic!("Not implemented")
    }

    async fn search(&self, db: &DbInstance) -> Result<Vec<Self>, Box<dyn error::Error>> {
        let query = doc! { "$or" : [{"id": self.id}] };
        let collection = self.get_collection(db);
        collection.search(query).await
    }

    fn get_collection_name(&self) -> &str {
        "system-setting"
    }
}

#[async_trait]
impl Database for TransactionItem {
    async fn insert(&self, _db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        panic!("Not implemented")
    }

    async fn update(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        let query = doc! { "id": self.id };
        let update = bson::to_bson(self).unwrap();
        let update = doc! { "$set" : update };
        let collection = self.get_collection(db);
        collection.update(query, update, true).await
    }

    async fn delete(&self, _db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        panic!("Not implemented")
    }

    async fn delete_all(&self, db: &DbInstance) -> Result<(), Box<dyn error::Error>> {
        let collection = self.get_collection(db);
        collection.delete_all().await
    }

    async fn search(&self, db: &DbInstance) -> Result<Vec<Self>, Box<dyn error::Error>> {
        let mut query = doc! { "id": { "$gt": 0 }};
        if self.user_name != "" && self.book_title != "" {
            query = doc! { "$or" : [{"user_id": self.user_id}, {"user_name": {"$regex": &self.user_name}}, {"book_id": &self.book_id}, {"book_title": {"$regex": &self.book_title}}] };
        } else if self.user_name != "" && self.book_title == "" {
            query = doc! { "$or" : [{"user_id": self.user_id}, {"user_name": {"$regex": &self.user_name}}, {"book_id": &self.book_id}] };
        } else if self.user_name == "" && self.book_title != "" {
            query = doc! { "$or" : [{"user_id": self.user_id}, {"book_id": &self.book_id}, {"book_title": {"$regex": &self.book_title}}] };
        } else if self.user_id != 0 || self.book_id != 0 {
            query = doc! { "$or" : [{"user_id": self.user_id}, {"book_id": &self.book_id}] };
        }

        let collection = self.get_collection(db);
        collection.search(query).await
    }

    fn get_collection_name(&self) -> &str {
        "transactions"
    }
}
