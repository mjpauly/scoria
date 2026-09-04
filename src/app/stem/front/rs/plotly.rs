//! Bindings to plotly's JS API.
//!
//! https://plotly.com/javascript/reference

use common::map_style::ColoredDataStream;
use jiff::{Span, Timestamp};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// React is the faster way to make a new plot and update it.
    ///
    /// https://plotly.com/javascript/plotlyjs-function-reference/
    #[wasm_bindgen(js_namespace = Plotly, js_name = react)]
    pub fn react(
        div_id: &str,
        data: &JsValue,
        layout: &JsValue,
        config: &JsValue,
    );

    /// Tear down a plot. Required on unmount: `responsive: true` registers
    /// a window resize listener that otherwise retains the graph div -- and
    /// through the detached DOM tree, everything else on the page (the
    /// analyze-page leak in doc/decimation/probe-results/churn.md).
    ///
    /// Takes the element, not its id: yew runs a child's effect destructors
    /// after the parent's DOM is already detached, so an id lookup fails
    /// there and Plotly throws. The exception unwinds through wasm with
    /// yew's hook context still borrowed, leaving the app dead (blank UI
    /// until reload). See purge_element.
    #[wasm_bindgen(catch, js_namespace = Plotly, js_name = purge)]
    fn purge_(gd: &web_sys::Element) -> Result<JsValue, JsValue>;
}

/// The graph div for `div_id`, looked up while it is still in the document
/// (i.e. inside the effect, not its destructor).
pub fn graph_div(div_id: &str) -> Option<web_sys::Element> {
    web_sys::window()?.document()?.get_element_by_id(div_id)
}

/// Purge a plot by element; works whether or not the element is still
/// attached. Errors are logged rather than propagated (see purge_).
pub fn purge_element(gd: &web_sys::Element) {
    if let Err(e) = purge_(gd) {
        tracing::error!("Plotly.purge failed: {e:?}");
    }
}

/// Generate iso 8601-style timestamps for plotly x axis.
pub fn time_to_str(zdt: &jiff::Zoned) -> String {
    zdt.datetime().to_string()
}

/// Generate iso 8601-style timestamps, but make sure there's a decimal. Without
/// a decimal plotly gives an "encountered bad format" error on the "%H:%M"
/// format specifier.
pub fn sec_to_timeofday<'a>(vals: impl Iterator<Item = &'a f64>) -> Value {
    let base: Timestamp = "2020-01-01T00:00:00Z".parse().unwrap();
    json!(vals
        .map(|x| format!("{:.2}", base + Span::new().seconds(*x as i64)))
        .collect::<Vec<_>>())
}

pub fn tickformat(colored_datastream: &ColoredDataStream) -> Value {
    if *colored_datastream == ColoredDataStream::TimeOfDay {
        Value::from("%H:%M")
    } else {
        Value::Null
    }
}

/// Extracted Plotly dark theme.
///
/// https://stackoverflow.com/questions/71721049/react-plotly-js-apply-dark-plotly-dark-theme
pub fn dark_template() -> Value {
    json!({
        "data": {
            "barpolar": [
                {
                    "marker": {
                        "line": {
                            "color": "rgb(17,17,17)",
                            "width": 0.5
                        },
                        "pattern": {
                            "fillmode": "overlay",
                            "size": 10,
                            "solidity": 0.2
                        }
                    },
                    "type": "barpolar"
                }
            ],
            "bar": [
                {
                    "error_x": {
                        "color": "#f2f5fa"
                    },
                    "error_y": {
                        "color": "#f2f5fa"
                    },
                    "marker": {
                        "line": {
                            "color": "rgb(17,17,17)",
                            "width": 0.5
                        },
                        "pattern": {
                            "fillmode": "overlay",
                            "size": 10,
                            "solidity": 0.2
                        }
                    },
                    "type": "bar"
                }
            ],
            "carpet": [
                {
                    "aaxis": {
                        "endlinecolor": "#A2B1C6",
                        "gridcolor": "#506784",
                        "linecolor": "#506784",
                        "minorgridcolor": "#506784",
                        "startlinecolor": "#A2B1C6"
                    },
                    "baxis": {
                        "endlinecolor": "#A2B1C6",
                        "gridcolor": "#506784",
                        "linecolor": "#506784",
                        "minorgridcolor": "#506784",
                        "startlinecolor": "#A2B1C6"
                    },
                    "type": "carpet"
                }
            ],
            "choropleth": [
                {
                    "colorbar": {
                        "outlinewidth": 0,
                        "ticks": ""
                    },
                    "type": "choropleth"
                }
            ],
            "contourcarpet": [
                {
                    "colorbar": {
                        "outlinewidth": 0,
                        "ticks": ""
                    },
                    "type": "contourcarpet"
                }
            ],
            "contour": [
                {
                    "colorbar": {
                        "outlinewidth": 0,
                        "ticks": ""
                    },
                    "colorscale": [
                        [
                            0.0,
                            "#0d0887"
                        ],
                        [
                            0.1111111111111111,
                            "#46039f"
                        ],
                        [
                            0.2222222222222222,
                            "#7201a8"
                        ],
                        [
                            0.3333333333333333,
                            "#9c179e"
                        ],
                        [
                            0.4444444444444444,
                            "#bd3786"
                        ],
                        [
                            0.5555555555555556,
                            "#d8576b"
                        ],
                        [
                            0.6666666666666666,
                            "#ed7953"
                        ],
                        [
                            0.7777777777777778,
                            "#fb9f3a"
                        ],
                        [
                            0.8888888888888888,
                            "#fdca26"
                        ],
                        [
                            1.0,
                            "#f0f921"
                        ]
                    ],
                    "type": "contour"
                }
            ],
            "heatmapgl": [
                {
                    "colorbar": {
                        "outlinewidth": 0,
                        "ticks": ""
                    },
                    "colorscale": [
                        [
                            0.0,
                            "#0d0887"
                        ],
                        [
                            0.1111111111111111,
                            "#46039f"
                        ],
                        [
                            0.2222222222222222,
                            "#7201a8"
                        ],
                        [
                            0.3333333333333333,
                            "#9c179e"
                        ],
                        [
                            0.4444444444444444,
                            "#bd3786"
                        ],
                        [
                            0.5555555555555556,
                            "#d8576b"
                        ],
                        [
                            0.6666666666666666,
                            "#ed7953"
                        ],
                        [
                            0.7777777777777778,
                            "#fb9f3a"
                        ],
                        [
                            0.8888888888888888,
                            "#fdca26"
                        ],
                        [
                            1.0,
                            "#f0f921"
                        ]
                    ],
                    "type": "heatmapgl"
                }
            ],
            "heatmap": [
                {
                    "colorbar": {
                        "outlinewidth": 0,
                        "ticks": ""
                    },
                    "colorscale": [
                        [
                            0.0,
                            "#0d0887"
                        ],
                        [
                            0.1111111111111111,
                            "#46039f"
                        ],
                        [
                            0.2222222222222222,
                            "#7201a8"
                        ],
                        [
                            0.3333333333333333,
                            "#9c179e"
                        ],
                        [
                            0.4444444444444444,
                            "#bd3786"
                        ],
                        [
                            0.5555555555555556,
                            "#d8576b"
                        ],
                        [
                            0.6666666666666666,
                            "#ed7953"
                        ],
                        [
                            0.7777777777777778,
                            "#fb9f3a"
                        ],
                        [
                            0.8888888888888888,
                            "#fdca26"
                        ],
                        [
                            1.0,
                            "#f0f921"
                        ]
                    ],
                    "type": "heatmap"
                }
            ],
            "histogram2dcontour": [
                {
                    "colorbar": {
                        "outlinewidth": 0,
                        "ticks": ""
                    },
                    "colorscale": [
                        [
                            0.0,
                            "#0d0887"
                        ],
                        [
                            0.1111111111111111,
                            "#46039f"
                        ],
                        [
                            0.2222222222222222,
                            "#7201a8"
                        ],
                        [
                            0.3333333333333333,
                            "#9c179e"
                        ],
                        [
                            0.4444444444444444,
                            "#bd3786"
                        ],
                        [
                            0.5555555555555556,
                            "#d8576b"
                        ],
                        [
                            0.6666666666666666,
                            "#ed7953"
                        ],
                        [
                            0.7777777777777778,
                            "#fb9f3a"
                        ],
                        [
                            0.8888888888888888,
                            "#fdca26"
                        ],
                        [
                            1.0,
                            "#f0f921"
                        ]
                    ],
                    "type": "histogram2dcontour"
                }
            ],
            "histogram2d": [
                {
                    "colorbar": {
                        "outlinewidth": 0,
                        "ticks": ""
                    },
                    "colorscale": [
                        [
                            0.0,
                            "#0d0887"
                        ],
                        [
                            0.1111111111111111,
                            "#46039f"
                        ],
                        [
                            0.2222222222222222,
                            "#7201a8"
                        ],
                        [
                            0.3333333333333333,
                            "#9c179e"
                        ],
                        [
                            0.4444444444444444,
                            "#bd3786"
                        ],
                        [
                            0.5555555555555556,
                            "#d8576b"
                        ],
                        [
                            0.6666666666666666,
                            "#ed7953"
                        ],
                        [
                            0.7777777777777778,
                            "#fb9f3a"
                        ],
                        [
                            0.8888888888888888,
                            "#fdca26"
                        ],
                        [
                            1.0,
                            "#f0f921"
                        ]
                    ],
                    "type": "histogram2d"
                }
            ],
            "histogram": [
                {
                    "marker": {
                        "pattern": {
                            "fillmode": "overlay",
                            "size": 10,
                            "solidity": 0.2
                        }
                    },
                    "type": "histogram"
                }
            ],
            "mesh3d": [
                {
                    "colorbar": {
                        "outlinewidth": 0,
                        "ticks": ""
                    },
                    "type": "mesh3d"
                }
            ],
            "parcoords": [
                {
                    "line": {
                        "colorbar": {
                            "outlinewidth": 0,
                            "ticks": ""
                        }
                    },
                    "type": "parcoords"
                }
            ],
            "pie": [
                {
                    "automargin": true,
                    "type": "pie"
                }
            ],
            "scatter3d": [
                {
                    "line": {
                        "colorbar": {
                            "outlinewidth": 0,
                            "ticks": ""
                        }
                    },
                    "marker": {
                        "colorbar": {
                            "outlinewidth": 0,
                            "ticks": ""
                        }
                    },
                    "type": "scatter3d"
                }
            ],
            "scattercarpet": [
                {
                    "marker": {
                        "colorbar": {
                            "outlinewidth": 0,
                            "ticks": ""
                        }
                    },
                    "type": "scattercarpet"
                }
            ],
            "scattergeo": [
                {
                    "marker": {
                        "colorbar": {
                            "outlinewidth": 0,
                            "ticks": ""
                        }
                    },
                    "type": "scattergeo"
                }
            ],
            "scattergl": [
                {
                    "marker": {
                        "line": {
                            "color": "#283442"
                        }
                    },
                    "type": "scattergl"
                }
            ],
            "scattermapbox": [
                {
                    "marker": {
                        "colorbar": {
                            "outlinewidth": 0,
                            "ticks": ""
                        }
                    },
                    "type": "scattermapbox"
                }
            ],
            "scatterpolargl": [
                {
                    "marker": {
                        "colorbar": {
                            "outlinewidth": 0,
                            "ticks": ""
                        }
                    },
                    "type": "scatterpolargl"
                }
            ],
            "scatterpolar": [
                {
                    "marker": {
                        "colorbar": {
                            "outlinewidth": 0,
                            "ticks": ""
                        }
                    },
                    "type": "scatterpolar"
                }
            ],
            "scatter": [
                {
                    "marker": {
                        "line": {
                            "color": "#283442"
                        }
                    },
                    "type": "scatter"
                }
            ],
            "scatterternary": [
                {
                    "marker": {
                        "colorbar": {
                            "outlinewidth": 0,
                            "ticks": ""
                        }
                    },
                    "type": "scatterternary"
                }
            ],
            "surface": [
                {
                    "colorbar": {
                        "outlinewidth": 0,
                        "ticks": ""
                    },
                    "colorscale": [
                        [
                            0.0,
                            "#0d0887"
                        ],
                        [
                            0.1111111111111111,
                            "#46039f"
                        ],
                        [
                            0.2222222222222222,
                            "#7201a8"
                        ],
                        [
                            0.3333333333333333,
                            "#9c179e"
                        ],
                        [
                            0.4444444444444444,
                            "#bd3786"
                        ],
                        [
                            0.5555555555555556,
                            "#d8576b"
                        ],
                        [
                            0.6666666666666666,
                            "#ed7953"
                        ],
                        [
                            0.7777777777777778,
                            "#fb9f3a"
                        ],
                        [
                            0.8888888888888888,
                            "#fdca26"
                        ],
                        [
                            1.0,
                            "#f0f921"
                        ]
                    ],
                    "type": "surface"
                }
            ],
            "table": [
                {
                    "cells": {
                        "fill": {
                            "color": "#506784"
                        },
                        "line": {
                            "color": "rgb(17,17,17)"
                        }
                    },
                    "header": {
                        "fill": {
                            "color": "#2a3f5f"
                        },
                        "line": {
                            "color": "rgb(17,17,17)"
                        }
                    },
                    "type": "table"
                }
            ]
        },
        "layout": {
            "annotationdefaults": {
                "arrowcolor": "#f2f5fa",
                "arrowhead": 0,
                "arrowwidth": 1
            },
            "autotypenumbers": "strict",
            "coloraxis": {
                "colorbar": {
                    "outlinewidth": 0,
                    "ticks": ""
                }
            },
            "colorscale": {
                "diverging": [
                    [
                        0,
                        "#8e0152"
                    ],
                    [
                        0.1,
                        "#c51b7d"
                    ],
                    [
                        0.2,
                        "#de77ae"
                    ],
                    [
                        0.3,
                        "#f1b6da"
                    ],
                    [
                        0.4,
                        "#fde0ef"
                    ],
                    [
                        0.5,
                        "#f7f7f7"
                    ],
                    [
                        0.6,
                        "#e6f5d0"
                    ],
                    [
                        0.7,
                        "#b8e186"
                    ],
                    [
                        0.8,
                        "#7fbc41"
                    ],
                    [
                        0.9,
                        "#4d9221"
                    ],
                    [
                        1,
                        "#276419"
                    ]
                ],
                "sequential": [
                    [
                        0.0,
                        "#0d0887"
                    ],
                    [
                        0.1111111111111111,
                        "#46039f"
                    ],
                    [
                        0.2222222222222222,
                        "#7201a8"
                    ],
                    [
                        0.3333333333333333,
                        "#9c179e"
                    ],
                    [
                        0.4444444444444444,
                        "#bd3786"
                    ],
                    [
                        0.5555555555555556,
                        "#d8576b"
                    ],
                    [
                        0.6666666666666666,
                        "#ed7953"
                    ],
                    [
                        0.7777777777777778,
                        "#fb9f3a"
                    ],
                    [
                        0.8888888888888888,
                        "#fdca26"
                    ],
                    [
                        1.0,
                        "#f0f921"
                    ]
                ],
                "sequentialminus": [
                    [
                        0.0,
                        "#0d0887"
                    ],
                    [
                        0.1111111111111111,
                        "#46039f"
                    ],
                    [
                        0.2222222222222222,
                        "#7201a8"
                    ],
                    [
                        0.3333333333333333,
                        "#9c179e"
                    ],
                    [
                        0.4444444444444444,
                        "#bd3786"
                    ],
                    [
                        0.5555555555555556,
                        "#d8576b"
                    ],
                    [
                        0.6666666666666666,
                        "#ed7953"
                    ],
                    [
                        0.7777777777777778,
                        "#fb9f3a"
                    ],
                    [
                        0.8888888888888888,
                        "#fdca26"
                    ],
                    [
                        1.0,
                        "#f0f921"
                    ]
                ]
            },
            "colorway": [
                "#3a87fe", // from iOS color picker (this line only)
                "#EF553B",
                "#00cc96",
                "#ab63fa",
                "#FFA15A",
                "#19d3f3",
                "#FF6692",
                "#B6E880",
                "#FF97FF",
                "#FECB52"
            ],
            "font": {
                "color": "#f2f5fa",
                "family": r#"ui-sans-serif, system-ui, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji""#,
            },
            "geo": {
                "bgcolor": "rgb(17,17,17)",
                "lakecolor": "rgb(17,17,17)",
                "landcolor": "rgb(17,17,17)",
                "showlakes": true,
                "showland": true,
                "subunitcolor": "#506784"
            },
            "hoverlabel": {
                "align": "left"
            },
            "hovermode": "closest",
            "mapbox": {
                "style": "dark"
            },
            "paper_bgcolor": "#000000",
            "plot_bgcolor": "#000000",
            "polar": {
                "angularaxis": {
                    "gridcolor": "#506784",
                    "linecolor": "#506784",
                    "ticks": ""
                },
                "bgcolor": "rgb(17,17,17)",
                "radialaxis": {
                    "gridcolor": "#506784",
                    "linecolor": "#506784",
                    "ticks": ""
                }
            },
            "scene": {
                "xaxis": {
                    "backgroundcolor": "rgb(17,17,17)",
                    "gridcolor": "#506784",
                    "gridwidth": 2,
                    "linecolor": "#506784",
                    "showbackground": true,
                    "ticks": "",
                    "zerolinecolor": "#C8D4E3"
                },
                "yaxis": {
                    "backgroundcolor": "rgb(17,17,17)",
                    "gridcolor": "#506784",
                    "gridwidth": 2,
                    "linecolor": "#506784",
                    "showbackground": true,
                    "ticks": "",
                    "zerolinecolor": "#C8D4E3"
                },
                "zaxis": {
                    "backgroundcolor": "rgb(17,17,17)",
                    "gridcolor": "#506784",
                    "gridwidth": 2,
                    "linecolor": "#506784",
                    "showbackground": true,
                    "ticks": "",
                    "zerolinecolor": "#C8D4E3"
                }
            },
            "shapedefaults": {
                "line": {
                    "color": "#f2f5fa"
                }
            },
            "sliderdefaults": {
                "bgcolor": "#C8D4E3",
                "bordercolor": "rgb(17,17,17)",
                "borderwidth": 1,
                "tickwidth": 0
            },
            "ternary": {
                "aaxis": {
                    "gridcolor": "#506784",
                    "linecolor": "#506784",
                    "ticks": ""
                },
                "baxis": {
                    "gridcolor": "#506784",
                    "linecolor": "#506784",
                    "ticks": ""
                },
                "bgcolor": "rgb(17,17,17)",
                "caxis": {
                    "gridcolor": "#506784",
                    "linecolor": "#506784",
                    "ticks": ""
                }
            },
            "title": {
                "x": 0.05
            },
            "updatemenudefaults": {
                "bgcolor": "#506784",
                "borderwidth": 0
            },
            "xaxis": {
                "automargin": true,
                "gridcolor": "#283442",
                "linecolor": "#506784",
                "ticks": "",
                "title": {
                    "standoff": 15
                },
                "zerolinecolor": "#283442",
                "zerolinewidth": 2
            },
            "yaxis": {
                "automargin": true,
                "gridcolor": "#283442",
                "linecolor": "#506784",
                "ticks": "",
                "title": {
                    "standoff": 15
                },
                "zerolinecolor": "#283442",
                "zerolinewidth": 2
            }
        }
    })
}
