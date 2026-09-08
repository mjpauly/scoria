use actix_web::http::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use actix_web::{
    http::header::ContentType, post, web, HttpRequest, HttpResponse, Responder,
};
use anyhow::{bail, Context, Result};
use secrecy::ExposeSecret;
use sqlx::PgPool;
use uuid::Uuid;

use crate::configuration::TurnstileSettings;
use crate::routes::static_files::{TEMPLATE_BOTTOM_FILE, TEMPLATE_TOP_FILE};

/// Subject sent by in-app problem reports. Those skip the Turnstile check
/// (the app can't render the widget) and get a CORS header so the device can
/// read the response.
const APP_REPORT_SUBJECT: &str = "App Problem Report wdiCCGLEBxcedhYxUOTqWhR";

const TURNSTILE_VERIFY_URL: &str =
    "https://challenges.cloudflare.com/turnstile/v0/siteverify";

#[derive(serde::Deserialize)]
pub struct FormData {
    subject: String,
    body: String,
    email: String,
    /// Hidden input injected by the Turnstile widget.
    #[serde(rename = "cf-turnstile-response")]
    turnstile_response: Option<String>,
}

#[derive(serde::Deserialize)]
struct SiteVerifyResponse {
    success: bool,
    #[serde(default, rename = "error-codes")]
    error_codes: Vec<String>,
}

/// Outcome of the Turnstile check, distinguishing a bad token from our own
/// failure to reach Cloudflare.
enum Verification {
    Passed,
    Rejected(Vec<String>),
}

async fn verify_turnstile(
    token: &str,
    remote_ip: Option<&str>,
    client: &reqwest::Client,
    settings: &TurnstileSettings,
) -> Result<Verification> {
    let mut params = vec![
        ("secret", settings.secret_key.expose_secret().as_str()),
        ("response", token),
    ];
    if let Some(ip) = remote_ip {
        params.push(("remoteip", ip));
    }
    let resp: SiteVerifyResponse = client
        .post(TURNSTILE_VERIFY_URL)
        .form(&params)
        .send()
        .await
        .context("Turnstile siteverify request failed")?
        .json()
        .await
        .context("Turnstile siteverify returned invalid JSON")?;
    Ok(if resp.success {
        Verification::Passed
    } else {
        Verification::Rejected(resp.error_codes)
    })
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
    req: HttpRequest,
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    http_client: web::Data<reqwest::Client>,
    turnstile: web::Data<TurnstileSettings>,
) -> impl Responder {
    let build_response_body = |content| {
        String::from(TEMPLATE_TOP_FILE)
            + r#"<p class="block">"#
            + content
            + r#"</p><p class="text-center"><a href="/">Main Page</a></p>"#
            + TEMPLATE_BOTTOM_FILE
    };

    let is_app_report = form.subject == APP_REPORT_SUBJECT;
    let reply = |mut builder: actix_web::HttpResponseBuilder, content| {
        builder.content_type(ContentType::html());
        if is_app_report {
            builder.insert_header((ACCESS_CONTROL_ALLOW_ORIGIN, "*"));
        }
        builder.body(build_response_body(content))
    };

    if !is_app_report {
        let Some(token) = form.turnstile_response.as_deref() else {
            return reply(
                HttpResponse::BadRequest(),
                "Please complete the verification challenge and try again.",
            );
        };
        let remote_ip =
            req.connection_info().realip_remote_addr().map(String::from);
        let verification = verify_turnstile(
            token,
            remote_ip.as_deref(),
            &http_client,
            &turnstile,
        )
        .await;
        match verification {
            Ok(Verification::Passed) => {}
            Ok(Verification::Rejected(codes)) => {
                println!("Turnstile rejected submission: {:?}", codes);
                return reply(
                    HttpResponse::BadRequest(),
                    "Verification failed. Please go back and try again.",
                );
            }
            Err(e) => {
                println!("Turnstile verification error: {:?}", e);
                return reply(
                    HttpResponse::InternalServerError(),
                    "Something went wrong...",
                );
            }
        }
    }

    match persist_feedback(form, pool).await {
        Ok(_) => {
            reply(HttpResponse::Ok(), "Submitted! Thanks for reaching out")
        }
        Err(e) => {
            println!("Failed to persist feedback: {:?}", e);
            reply(
                HttpResponse::InternalServerError(),
                "Something went wrong...",
            )
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
