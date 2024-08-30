use std::str::FromStr;

use common::bool_expr::BoolExpr;
use common::pin::pin_predicate::{PinStringPred, PinStringPredKind};
use common::pin::PinFilters;
use strum::IntoEnumIterator;
use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yewdux::prelude::*;

use crate::components::{
    SECONDARY_BUTTON_STYLE, SELECT_STYLE, SHORT_INPUT_STYLE,
    TOGGLE_SWITCH_STYLE,
};
use crate::ui_state::FrontState;
use crate::unwrapping::{unwrap_option_or_log, unwrap_result_or_log};

#[function_component]
pub fn PlacesFilterList() -> Html {
    let editing = use_selector(|s: &FrontState| s.pin_settings.editing_filter);
    let filters = use_selector(|s: &FrontState| s.pin_settings.filters.clone());
    let dispatch = Dispatch::<FrontState>::new();
    let editing_onchange = dispatch.reduce_mut_callback_with(
        |s: &mut FrontState, new_editing: Option<usize>| {
            s.pin_settings.editing_filter = new_editing;
        },
    );
    let filters_onchange = dispatch.reduce_mut_callback_with(
        |s: &mut FrontState, new_filters: PinFilters| {
            s.pin_settings.filters = new_filters;
        },
    );
    html! {
        <GenericPlacesFilterList
            filters={(*filters).clone()}
            editing={*editing}
            filters_onchange={filters_onchange}
            editing_onchange={editing_onchange}
        />
    }
}

#[derive(Properties, PartialEq)]
pub struct PlacesFilterListProps {
    pub filters: PinFilters,
    pub editing: Option<usize>,
    pub filters_onchange: Callback<PinFilters>,
    pub editing_onchange: Callback<Option<usize>>,
}

#[function_component]
pub fn GenericPlacesFilterList(p: &PlacesFilterListProps) -> Html {
    let editing = p.editing;
    let exprs = &p.filters;
    let small_button_style = format!("px-3 py-1.5 {}", SECONDARY_BUTTON_STYLE);

    let exprs_html =
        {
            let editing_onchange = p.editing_onchange.clone();
            let exprs_onchange = p.filters_onchange.clone();
            exprs.clone().into_iter().enumerate().map(
                move |(i, (active, expr))| {
                    let is_editing = Some(i) == editing;
                    let expr_html = if is_editing {
                        let onchange = {
                            let exprs = exprs.clone();
                            let exprs_onchange = exprs_onchange.clone();
                            Callback::from(
                                move |new_expr: BoolExpr<PinStringPred>| {
                                    let mut new_exprs = exprs.clone();
                                    new_exprs[i] = (active, new_expr);
                                    exprs_onchange.emit(new_exprs);
                                },
                            )
                        };
                        html! {
                            <FilterExprEditor
                                expr={expr.clone()}
                                onchange={onchange}
                            />
                        }
                    } else {
                        html! {
                            <FilterExprViewer expr={expr.clone()} />
                        }
                    };
                    let on_toggle = {
                        let exprs = exprs.clone();
                        let exprs_onchange = exprs_onchange.clone();
                        Callback::from(move |_e: MouseEvent| {
                            let mut new_exprs = exprs.clone();
                            new_exprs[i].0 = !active;
                            exprs_onchange.emit(new_exprs);
                        })
                    };
                    let edit_onclick = {
                        let editing_onchange = editing_onchange.clone();
                        Callback::from(move |_e: MouseEvent| {
                            if is_editing {
                                editing_onchange.emit(None);
                            } else {
                                editing_onchange.emit(Some(i));
                            }
                        })
                    };
                    let duplicate_onclick = {
                        let exprs = exprs.clone();
                        let exprs_onchange = exprs_onchange.clone();
                        Callback::from(move |_e: MouseEvent| {
                            let mut new_exprs = exprs.clone();
                            new_exprs.insert(i, exprs[i].clone());
                            exprs_onchange.emit(new_exprs);
                        })
                    };
                    let remove_onclick = {
                        let exprs = exprs.clone();
                        let exprs_onchange = exprs_onchange.clone();
                        let editing_onchange = editing_onchange.clone();
                        Callback::from(move |_e: MouseEvent| {
                            let mut new_exprs = exprs.clone();
                            new_exprs.remove(i);
                            exprs_onchange.emit(new_exprs);
                            editing_onchange.emit(None);
                        })
                    };
                    html! {
                        <div class="flex gap-4 items-center px-4 py-2 \
                        bg-neutral-900 w-min rounded-lg">
                            <div class="flex flex-col gap-2 justify-center \
                            items-center">
                                <div class="relative h-6">
                                    <input
                                        type="checkbox"
                                        checked={active}
                                        onclick={on_toggle}
                                        class={TOGGLE_SWITCH_STYLE}
                                    />
                                </div>
                                <button
                                    onclick={edit_onclick}
                                    class={small_button_style.clone()}
                                >
                                    if is_editing {
                                        {"Done"}
                                    } else {
                                        {"Edit"}
                                    }
                                </button>
                                if is_editing {
                                    <button
                                        onclick={duplicate_onclick}
                                        class={small_button_style.clone()}
                                    >
                                        {"Clone"}
                                    </button>
                                    <button
                                        onclick={remove_onclick}
                                        class={small_button_style.clone()}
                                    >
                                        {"Remove"}
                                    </button>
                                }
                            </div>
                            {expr_html}
                        </div>
                    }
                },
            )
        };
    let new_filter_onclick = {
        let exprs_onchange = p.filters_onchange.clone();
        let exprs = exprs.clone();
        Callback::from(move |_e: MouseEvent| {
            let mut new_exprs = exprs.clone();
            new_exprs.push((false, PinStringPred::default().into()));
            exprs_onchange.emit(new_exprs);
        })
    };

    html! {
        <div class="px-4 w-min flex flex-col gap-2 items-start mx-auto">
            {for exprs_html}
            <button onclick={new_filter_onclick}>
                <div class="rounded-lg p-2 bg-neutral-800 w-min">
                    <Icon icon_id={IconId::BootstrapPlusLg}
                        class="h-5 w-5 text-neutral-400" />
                </div>
            </button>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct FilterExprViewerProps {
    pub expr: BoolExpr<PinStringPred>,
}

#[function_component]
fn FilterExprViewer(p: &FilterExprViewerProps) -> Html {
    match &p.expr {
        BoolExpr::And(left, right) | BoolExpr::Or(left, right) => {
            let is_and = matches!(p.expr, BoolExpr::And(_, _));
            let colors = match is_and {
                true => "border-cyan-800",
                false => "border-green-800",
            };
            html! {
                <div class={format!("border-2 rounded-lg \
                    py-2 px-4 flex gap-4 items-center {}", colors)}>
                    <span>
                        if is_and {
                            {"And"}
                        } else {
                            {"Or"}
                        }
                    </span>
                    <div class="flex flex-col gap-2 items-start">
                        <FilterExprViewer expr={(**left).clone()} />
                        <FilterExprViewer expr={(**right).clone()} />
                    </div>
                </div>
            }
        }
        BoolExpr::Not(inner) => {
            html! {
                <div class="border-2 border-red-800 rounded-lg py-2 px-4 \
                    flex gap-4 items-center">
                    <span>{"Not"}</span>
                    <FilterExprViewer expr={(**inner).clone()} />
                </div>
            }
        }
        BoolExpr::Predicate(pred) => {
            html! {
                <div class="border-2 border-neutral-500 rounded-lg py-2 px-4 \
                    flex gap-4 items-center bg-neutral-900">
                    <PredViewer pred={pred.clone()} />
                </div>
            }
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct PredViewerProps {
    pub pred: PinStringPred,
}

#[function_component]
fn PredViewer(p: &PredViewerProps) -> Html {
    html! {
        <div class="flex flex-col gap-2 w-min">
            <span> {p.pred.kind.to_string()} </span>
            <span> {"\""} {p.pred.value.clone()} {"\""} </span>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct FilterExprEditorProps {
    pub expr: BoolExpr<PinStringPred>,
    pub onchange: Callback<BoolExpr<PinStringPred>>,
}

#[function_component]
fn FilterExprEditor(p: &FilterExprEditorProps) -> Html {
    let invert = {
        let self_expr = p.expr.clone();
        let onchange = p.onchange.clone();
        Callback::from(move |_e: MouseEvent| {
            let new_self = match &self_expr {
                BoolExpr::Not(inner) => (**inner).clone(),
                _ => BoolExpr::Not(self_expr.clone().into()),
            };
            onchange.emit(new_self);
        })
    };
    let small_button_style = format!("px-3 py-1.5 {}", SECONDARY_BUTTON_STYLE);
    let invert_button = html! {
        <button class={small_button_style.clone()} onclick={invert} >
            {"Invert"}
        </button>
    };
    let branch = {
        let self_expr = p.expr.clone();
        let onchange = p.onchange.clone();
        Callback::from(move |_e: MouseEvent| {
            let new_self = BoolExpr::Or(
                self_expr.clone().into(),
                PinStringPred::default().into(),
            );
            onchange.emit(new_self);
        })
    };
    let branch_button = html! {
        <button class={small_button_style.clone()} onclick={branch} >
            {"Branch"}
        </button>
    };
    match &p.expr {
        BoolExpr::And(left, right) | BoolExpr::Or(left, right) => {
            let is_and = matches!(p.expr, BoolExpr::And(_, _));
            let onchange_maker = |left_changed: bool| {
                let left = left.clone();
                let right = right.clone();
                let onchange = p.onchange.clone();
                Callback::from(move |new_expr: BoolExpr<PinStringPred>| {
                    let new_self = match (is_and, left_changed) {
                        (true, true) => {
                            BoolExpr::And(new_expr.into(), right.clone())
                        }
                        (true, false) => {
                            BoolExpr::And(left.clone(), new_expr.into())
                        }
                        (false, true) => {
                            BoolExpr::Or(new_expr.into(), right.clone())
                        }
                        (false, false) => {
                            BoolExpr::Or(left.clone(), new_expr.into())
                        }
                    };
                    onchange.emit(new_self);
                })
            };
            let remove = |is_left| {
                let left = left.clone();
                let right = right.clone();
                let onchange = p.onchange.clone();
                Callback::from(move |_e: MouseEvent| {
                    let new_self = match is_left {
                        true => (*right).clone(),
                        false => (*left).clone(),
                    };
                    onchange.emit(new_self);
                })
            };
            let switch_and_or = {
                let left = left.clone();
                let right = right.clone();
                let onchange = p.onchange.clone();
                Callback::from(move |_e: MouseEvent| {
                    let new_self = match is_and {
                        true => BoolExpr::Or(left.clone(), right.clone()),
                        false => BoolExpr::And(left.clone(), right.clone()),
                    };
                    onchange.emit(new_self);
                })
            };
            let swap_left_right = {
                let left = left.clone();
                let right = right.clone();
                let onchange = p.onchange.clone();
                Callback::from(move |_e: MouseEvent| {
                    let new_self = match is_and {
                        true => BoolExpr::And(right.clone(), left.clone()),
                        false => BoolExpr::Or(right.clone(), left.clone()),
                    };
                    onchange.emit(new_self);
                })
            };
            let colors = match is_and {
                true => "border-cyan-800",
                false => "border-green-800",
            };
            html! {
                <div class={format!("border-2 rounded-lg \
                    py-2 px-4 flex gap-4 items-center {}", colors)}>
                    <span>
                        if is_and {
                            {"And"}
                        } else {
                            {"Or"}
                        }
                    </span>
                    <div class="flex flex-col gap-2 items-start">
                        <div class="flex gap-2">
                            <FilterExprEditor
                                expr={(**left).clone()}
                                onchange={onchange_maker(true)}
                            />
                            <button onclick={remove(true)} >
                                <Icon icon_id={IconId::BootstrapXCircle}
                                    class="h-5 w-5 text-neutral-500" />
                            </button>
                        </div>
                        <div class="flex gap-2">
                            <FilterExprEditor
                                expr={(**right).clone()}
                                onchange={onchange_maker(false)}
                            />
                            <button onclick={remove(false)} >
                                <Icon icon_id={IconId::BootstrapXCircle}
                                    class="h-5 w-5 text-neutral-500" />
                            </button>
                        </div>
                        <div class="flex gap-2">
                            <button
                                class={small_button_style.clone()}
                                onclick={switch_and_or}
                            >
                                if is_and {
                                    {"Or"}
                                } else {
                                    {"And"}
                                }
                            </button>
                            <button
                                class={small_button_style.clone()}
                                onclick={swap_left_right}
                            >
                                {"Swap"}
                            </button>
                            {invert_button}
                            {branch_button}
                        </div>
                    </div>
                </div>
            }
        }
        BoolExpr::Not(inner) => {
            let onchange = {
                let onchange = p.onchange.clone();
                Callback::from(move |new_expr: BoolExpr<PinStringPred>| {
                    let new_self = match &new_expr {
                        // child changed to Not, remove double Not
                        BoolExpr::Not(inner) => (**inner).clone(),
                        _ => BoolExpr::Not(new_expr.into()),
                    };
                    onchange.emit(new_self);
                })
            };
            html! {
                <div class="border-2 border-red-800 rounded-lg py-2 px-4 \
                    flex gap-4 items-center">
                    <span>{"Not"}</span>
                    <div class="flex flex-col gap-2 items-start">
                        <FilterExprEditor
                            expr={(**inner).clone()}
                            onchange={onchange}
                        />
                        <div class="flex gap-2">
                            {invert_button}
                            {branch_button}
                        </div>
                    </div>
                </div>
            }
        }
        BoolExpr::Predicate(pred) => {
            let onchange = {
                let onchange = p.onchange.clone();
                Callback::from(move |new_pred: PinStringPred| {
                    onchange.emit(new_pred.into());
                })
            };
            html! {
                <div class="border-2 border-neutral-500 rounded-lg py-2 px-4 \
                    flex gap-4 items-center">
                    <div class="flex flex-col gap-2 items-start">
                        <Pred pred={pred.clone()} onchange={onchange} />
                        <div class="flex gap-2">
                            {invert_button}
                            {branch_button}
                        </div>
                    </div>
                </div>
            }
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct PredEditorProps {
    pub pred: PinStringPred,
    pub onchange: Callback<PinStringPred>,
}

#[function_component]
fn Pred(p: &PredEditorProps) -> Html {
    let kind_options = PinStringPredKind::iter().map(|x| {
        html! {
            <option selected={x == p.pred.kind}>
                {x.to_string()}
            </option>
        }
    });
    let kind_onchange = {
        let pred = p.pred.clone();
        let onchange = p.onchange.clone();
        Callback::from(move |e: Event| {
            let elem: web_sys::HtmlSelectElement =
                unwrap_option_or_log!(e.target_dyn_into());
            let val: &str = &elem.value();
            let mut pred = pred.clone();
            pred.kind = unwrap_result_or_log!(PinStringPredKind::from_str(val));
            onchange.emit(pred);
        })
    };
    let value_onchange = {
        let pred = p.pred.clone();
        let onchange = p.onchange.clone();
        Callback::from(move |e: Event| {
            // TODO: truncate long inputs?
            let elem: web_sys::HtmlInputElement = e.target_dyn_into().unwrap();
            let mut pred = pred.clone();
            pred.value = elem.value();
            onchange.emit(pred);
        })
    };
    html! {
        <div class="flex flex-col gap-2 w-min">
            <select onchange={kind_onchange}
                class={format!("{}", SELECT_STYLE)}>
                {for kind_options}
            </select>
            <input class={format!("{} px-1 w-full", SHORT_INPUT_STYLE)}
                value={p.pred.value.clone()}
                onchange={value_onchange}
            />
        </div>
    }
}
