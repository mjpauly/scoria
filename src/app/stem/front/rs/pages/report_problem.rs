//! Page for reporting problems (e.g. bugs) in the app to the developers.

use common::ToBack;
use gloo_net::http::Request;
use yew::prelude::*;
use yew_icons::{Icon, IconId};
// use yew_router::prelude::*;
use js_sys::encode_uri_component;
use obfstr::obfstr;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yewdux::prelude::*;

use crate::components::{DoneButton, MainSettingsButton};
use crate::components::{HomeBarSpacer, TopNav};
use crate::components::{InfoMessage, TOGGLE_SWITCH_STYLE};
use crate::ui_state::{BackState, FrontState};
use crate::websocket::WebsocketService;

#[function_component]
pub fn ReportProblem() -> Html {
    let dispatch = Dispatch::<FrontState>::new();
    let problem_report =
        use_selector(|s: &FrontState| s.problem_report.clone());
    let last_error = use_selector(|s: &BackState| s.last_logged_error.clone());

    let email_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: Event| {
            let elem: HtmlInputElement = e.target_dyn_into().unwrap();
            s.problem_report.email = elem.value();
        },
    );

    let body_onchange = dispatch.reduce_mut_callback_with(
        move |s: &mut FrontState, e: Event| {
            let elem: HtmlTextAreaElement = e.target_dyn_into().unwrap();
            s.problem_report.body = elem.value();
        },
    );

    let attach = problem_report.attach_log;
    let attach_on_click =
        dispatch.reduce_mut_callback(move |s: &mut FrontState| {
            s.problem_report.attach_log = !s.problem_report.attach_log;
        });

    let view_log = use_state(|| false);
    let view_log_onclick = {
        let view_log = view_log.clone();
        Callback::from(move |_| view_log.set(!*view_log))
    };

    // Tell backend that we reviewed the last logged error
    let wss = use_context::<WebsocketService>().unwrap();
    let dismiss_onclick = {
        let wss = wss.clone();
        Callback::from(move |_| wss.send_msg(ToBack::ReviewedLastError))
    };

    let can_send_log = last_error.is_some() && attach;
    let body_filled = !problem_report.body.is_empty();
    // a report should contain either a non-empty log file or a non-empty body
    let can_send = can_send_log || body_filled;

    let show_send_success = use_state(|| false);
    let show_send_error = use_state(|| false);

    let send_onclick = {
        let last_error = last_error.clone();
        let problem_report = problem_report.clone();
        let show_send_success = show_send_success.clone();
        let show_send_error = show_send_error.clone();
        Callback::from(move |_| {
            if can_send {
                wss.send_msg(ToBack::ReviewedLastError);

                let email = problem_report.email.clone();
                let body = if let (Some((err, _)), true) =
                    (&*last_error, problem_report.attach_log)
                {
                    // append log messages to report body
                    format!("{}\n\n--LOG--\n\n{err}", problem_report.body)
                } else if problem_report.attach_log {
                    format!("{}\n\n--LOG--\n\nNo errors.", problem_report.body)
                } else {
                    problem_report.body.clone()
                };
                // Subject must be set to this string for the server to respond
                // with a CORS header that allows the browser to read the
                // response. See website/aft/routes/contact.rs. Makes it more
                // difficult for third parties to abuse CORS.
                obfstr! {
                    let subject = "App Problem Report wdiCCGLEBxcedhYxUOTqWhR";
                }

                let email: String = encode_uri_component(&email).into();
                let body: String = encode_uri_component(&body).into();
                let subject: String = encode_uri_component(subject).into();
                // server expects form data in the body of the request
                let reqbody =
                    format!("subject={subject}&body={body}&email={email}");

                let dispatch = dispatch.clone();
                let show_send_success = show_send_success.clone();
                let show_send_error = show_send_error.clone();
                yew::platform::spawn_local(async move {
                    let result =
                        Request::post(obfstr!("https://scoria.info/contact"))
                            .header(
                                "Content-Type",
                                "application/x-www-form-urlencoded",
                            )
                            .body(reqbody)
                            .send()
                            .await;
                    if let Ok(resp) = result {
                        if resp.ok() {
                            dispatch.reduce_mut(|s: &mut FrontState| {
                                s.problem_report = Default::default();
                            });
                            show_send_success.set(true);
                        } else {
                            show_send_error.set(true);
                        }
                    } else {
                        show_send_error.set(true);
                    }
                });
            }
        })
    };
    html! {
        <>
            <TopNav>
                <MainSettingsButton />
                <DoneButton />
            </TopNav>
            <div
                class="grow overflow-scroll h-0 px-4 w-full max-w-prose mx-auto"
            >
                <h1 class="font-bold text-3xl text-left my-4 px-2">
                    {"Report a Problem"}
                </h1>

                if *show_send_success {
                    <p class="mt-6 px-2 text-left block">
                        {"Report sent. Thank you for helping to improve
                        Scoria!"}
                    </p>
                } else if *show_send_error {
                    <p class="mt-6 px-2 text-left block">
                        {"Something went wrong. Please try again later."}
                    </p>
                } else {

                if let Some((_, reviewed)) = &*last_error {
                    if !reviewed {
                    // last error not reviewed
                    <InfoMessage>
                        <div class="flex flex-col items-start">
                            <span class="text-left">
                                {"Errors were found in your app log. Please
                                consider submitting it to help improve Scoria. A
                                description is optional in this case."}
                            </span>
                            <button
                                class="py-1 pr-4 text-primary font-bold"
                                onclick={dismiss_onclick}
                            >
                                {"Dismiss"}
                            </button>
                        </div>
                    </InfoMessage>
                    }
                }

                // description
                <p class="pb-2 px-2 text-left mt-4">
                    {"To help you more effectively, your app's log file can be
                    attached to this message. The log only contains details
                    about errors your app has come across within a one-hour
                    window. Your report will be securely sent over an encrypted
                    channel."}
                </p>

                // email
                <label for="email" class="mt-2 block">
                    <input
                        id="email"
                        placeholder="Your email (optional)"
                        onchange={email_onchange}
                        value={problem_report.email.clone()}
                        class="block w-full rounded-md bg-neutral-800 px-4
                        py-2 text-neutral-300"
                    />
                </label>

                // body
                <label for="body" class="mt-2 block">
                    <textarea
                        id="body"
                        placeholder="Description"
                        onchange={body_onchange}
                        value={problem_report.body.clone()}
                        class="block w-full h-40 rounded-md
                        bg-neutral-800 px-4 py-2 text-neutral-300">
                    </textarea>
                </label>

                // settings card
                <div class="mt-2 bg-neutral-900 rounded-lg px-4">
                // attach log
                <div
                    class="flex items-center justify-between py-2 border-b
                    border-neutral-800"
                >
                    <label for="attach_log">
                        {"Attach app log"}
                    </label>
                    <div class="relative ml-4 h-6">
                    <input type="checkbox" id="attach_log"
                        checked={attach}
                        onclick={attach_on_click}
                        class={TOGGLE_SWITCH_STYLE}
                    />
                    </div>
                </div>

                // view log
                <button
                    onclick={view_log_onclick}
                    class="py-2 w-full text-left flex items-center
                    justify-between"
                >
                    <label>{"View app log"}</label>
                    <Icon
                        icon_id={
                            if *view_log {
                                IconId::BootstrapChevronDown
                            } else {
                                IconId::BootstrapChevronLeft
                            }
                        }
                        class="h-4 w-4 text-neutral-500"
                    />
                </button>

                if *view_log {
                    <p
                        class="pb-2 text-neutral-400 font-mono
                        whitespace-pre-wrap text-left text-xs"
                    >
                        if let Some((err, _)) = &*last_error {
                            {err}
                        } else {
                            {"No errors."}
                        }
                    </p>
                }
                </div>

                // send
                <button
                    class="mt-4 block w-full rounded-lg bg-neutral-900 px-4 py-2
                    mb-8 text-left text-primary disabled:opacity-75"
                    disabled={!can_send}
                    onclick={send_onclick}
                >
                    {"Send Report"}
                </button>
                }

            </div>

            <HomeBarSpacer />
        </>
    }
}
