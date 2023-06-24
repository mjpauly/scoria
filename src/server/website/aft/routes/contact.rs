use actix_web::{
    http::header::ContentType, post, web, HttpResponse, Responder,
};
use anyhow::{bail, Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::routes::static_files::{TEMPLATE_BOTTOM_FILE, TEMPLATE_TOP_FILE};

#[derive(serde::Deserialize)]
pub struct FormData {
    subject: String,
    body: String,
    email: String,
}

pub fn parse(mut s: String, max_len: usize) -> Result<String> {
    let is_empty_or_whitespace = s.trim().is_empty();
    if is_empty_or_whitespace {
        bail!("Empty field");
    }
    s.truncate(max_len); // truncate large strings
    let forbidden_character_replacements = [
        ("/", "[f_slash]"),
        ("(", "[o_paren]"),
        (")", "[c_paren]"),
        ("\"", "[quote]"),
        ("<", "[o_angle]"),
        (">", "[c_angle]"),
        ("\\", "[b_slash]"),
        ("{", "[o_curly]"),
        ("}", "[c_curly]"),
    ];
    // Replace risky characters with a text version.
    // This is not necessary since we're using prepared SQL statements, but it
    // can't hurt to be extra careful.
    for replacement_pair in forbidden_character_replacements {
        s = s.replace(replacement_pair.0, replacement_pair.1);
    }
    Ok(s)
}

#[post("/contact")]
pub async fn contact_form_submitted(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    let build_response_body = |content| {
        String::from(TEMPLATE_TOP_FILE)
            + r#"<p class="block">"#
            + content
            + r#"</p><p class="text-center"><a href="/">Main Page</a></p>"#
            + TEMPLATE_BOTTOM_FILE
    };
    let ok_response = "Submitted! Thanks for reaching out";
    let err_response = "Something went wrong...";
    match persist_feedback(form, pool).await {
        Ok(_) => HttpResponse::Ok()
            .content_type(ContentType::html())
            .body(build_response_body(ok_response)),
        Err(e) => {
            println!("Failed to persist feedback: {:?}", e);
            HttpResponse::InternalServerError()
                .content_type(ContentType::html())
                .body(build_response_body(err_response))
        }
    }
}

async fn persist_feedback(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> Result<()> {
    // Max 256 characters for the subject and email fields, and 2048 for the
    // body. Longer strings are truncated.
    let subject = parse(form.subject.clone(), 256).context("Empty subject")?;
    let body = parse(form.body.clone(), 2048).context("Empty body")?;
    let email = match parse(form.email.clone(), 256) {
        Ok(s) => Some(s),
        Err(_) => None, // email is optional, change to None on error
    };
    /*
    // Not sure how to use sqlx offline mode with Bazel, since `sqlx prepare`
    // is invoked as a cargo command, which means duplicating the Bazel build
    // system just for cargo. So we don't use the sqlx query macros, unless it's
    // for local testing outside of docker.
    sqlx::query!(
        r#"
        INSERT INTO feedback (id, subject, body, email, source, sent_at)
        VALUES ($1, $2, $3, $4, 'website', $5)
        "#,
        Uuid::new_v4(),
        subject,
        body,
        email,
        time::OffsetDateTime::now_utc(),
    )
    */
    sqlx::query(
        r#"
        INSERT INTO feedback (id, subject, body, email, source, sent_at)
        VALUES ($1, $2, $3, $4, 'website', $5)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(subject)
    .bind(body)
    .bind(email)
    .bind(time::OffsetDateTime::now_utc())
    .execute(&**pool)
    .await
    .context("Failed to persist feedback in the database")?;
    Ok(())
}
