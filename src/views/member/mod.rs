use crate::views::path::Path;
use actix_web::web;
mod member;

pub fn member_factory(app: &mut web::ServiceConfig) {
    let base_path: Path = Path {
        prefix: String::from("/member"),
    };
    app.route(
        &base_path.define(String::from("/borrowed-books")),
        web::get().to(member::borrowed_books),
    )
    .route(
        &base_path.define(String::from("/search-page")),
        web::get().to(member::load_search),
    )
    .route(
        &base_path.define(String::from("/home-page")),
        web::get().to(member::load_home),
    );
}
