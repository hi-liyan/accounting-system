use axum::response::{Html, Redirect};
use askama::Template;

use crate::middleware::{CurrentUser, OptionalCurrentUser};

#[derive(Template)]
#[template(path = "dashboard/index.html")]
struct DashboardTemplate {
    user: CurrentUser,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    user: Option<CurrentUser>,
}

pub async fn dashboard(current_user: CurrentUser) -> Html<String> {
    let template = DashboardTemplate {
        user: current_user,
    };
    Html(template.render().unwrap())
}

pub async fn index(OptionalCurrentUser(user): OptionalCurrentUser) -> Result<Html<String>, Redirect> {
    match user {
        Some(_) => Err(Redirect::to("/dashboard")),
        None => {
            let template = IndexTemplate { user: None };
            Ok(Html(template.render().unwrap()))
        }
    }
}