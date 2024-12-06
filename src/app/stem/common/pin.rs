//! A place marked on the map with an emoji icon and other user-annotated
//! fields.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::bool_expr::BoolExpr;
use crate::validation::{trunc_to_char, trunc_to_grapheme};
use crate::LngLat;
use pin_predicate::{PinStringPred, PinStringPredKind};

const URL_PREFIX: &str = "scoria://place?"; // scheme, host, and query param ?

// Maximum number of characters (code points) for various fields
const ICON_MAX_GRAPHEMES: usize = 5; // max number of graphemes
const ICON_MAX_CHARS: usize = 32; // max number of code points (some emoji have
                                  // up to 4 code points)
const NAME_MAX_CHARS: usize = 128;
const LIST_MAX_CHARS: usize = 128;
const TAG_KEY_MAX_CHARS: usize = 128;
const TAG_VAL_MAX_CHARS: usize = 1024;

// filter is active status, and the filter expression itself
pub type PinFilters = Vec<(bool, BoolExpr<PinStringPred>)>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PinSettings {
    pub filters: PinFilters,
    pub editing_filter: Option<usize>, // index of the filter being edited
    pub show_filters: bool,
    pub place_detailed_view: Option<Pin>, // the pin to show a detailed
                                          // breakdown for in the Place tab
}

impl Default for PinSettings {
    fn default() -> Self {
        Self {
            filters: vec![(
                true,
                BoolExpr::Not(
                    PinStringPred {
                        kind: PinStringPredKind::InList,
                        value: "Hidden".into(),
                    }
                    .into(),
                ),
            )],
            editing_filter: None,
            show_filters: false,
            place_detailed_view: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pin {
    pub id: Option<i64>,
    pub lnglat: LngLat, // location of the icon / name
    pub name: String,
    pub icon: String,                  // emoji icon
    pub lists: Vec<String>,            // json array of strings
    pub tags: Vec<(String, String)>,   // key value pairs
    pub boundary: Option<Vec<LngLat>>, // currently unused
}

/// Default to a star emoji and "Untitled" name.
impl Default for Pin {
    fn default() -> Self {
        Self {
            id: Default::default(),
            lnglat: Default::default(),
            name: "".into(),
            icon: "⭐️".into(),
            lists: Default::default(),
            tags: Default::default(),
            boundary: Default::default(),
        }
    }
}

impl Pin {
    pub fn set_name(&mut self, mut s: String) {
        trunc_to_char(&mut s, NAME_MAX_CHARS);
        self.name = s;
    }

    /// Set the icon, truncating it to its max characters.
    ///
    /// Returns an error if the new value is all whitespace, which will
    /// confusingly result in an invisible icon on the map
    pub fn set_icon(&mut self, mut s: String) -> Result<()> {
        // Truncate to max number of unicode code points
        trunc_to_char(&mut s, ICON_MAX_CHARS);
        // Emoji might not be followed by variation selectors, so we also
        // truncate to a max number of graphemes
        trunc_to_grapheme(&mut s, ICON_MAX_GRAPHEMES);
        if s.trim().is_empty() {
            bail!("Only whitespace, which would be invisible.")
        }
        self.icon = s;
        Ok(())
    }

    /// Set the lnglat, returning an error if the lnglat is invalid.
    pub fn set_lnglat(&mut self, newlnglat: LngLat) -> Result<()> {
        if !newlnglat.is_valid() {
            bail!("Lnglat not on the globe.")
        }
        self.lnglat = newlnglat;
        Ok(())
    }

    pub fn push_list(&mut self, mut s: String) {
        trunc_to_char(&mut s, LIST_MAX_CHARS);
        self.lists.push(s);
    }

    pub fn set_lists(&mut self, lists: Vec<String>) {
        for list in lists.into_iter() {
            self.push_list(list);
        }
    }

    pub fn update_key_at(&mut self, index: usize, mut s: String) {
        trunc_to_char(&mut s, TAG_KEY_MAX_CHARS);
        self.tags[index].0 = s;
    }

    pub fn update_val_at(&mut self, index: usize, mut s: String) {
        trunc_to_char(&mut s, TAG_VAL_MAX_CHARS);
        self.tags[index].1 = s;
    }

    pub fn set_tags(&mut self, tags: Vec<(String, String)>) {
        for (mut k, mut v) in tags.into_iter() {
            trunc_to_char(&mut k, TAG_KEY_MAX_CHARS);
            trunc_to_char(&mut v, TAG_VAL_MAX_CHARS);
            self.tags.push((k, v));
        }
    }

    /// Produce a scoria url scheme containing the pin data.
    ///
    /// Example:
    ///
    /// ```text
    /// scoria://place?name=Ferry+Building&lng=-122.39339582391952&lat=\
    /// 37.79552680112931&icon=%E2%9B%B4%EF%B8%8F
    /// ```
    ///
    /// Brackets that indicates nesting are manually percent encoded. This
    /// avoids other software attempting to then percent encode the whole url
    /// because it detects the brackets (e.g. Notes), which messes up things
    /// that are already encoded, like emojis. Parsing the url thus requires
    /// non-strict mode for serde_qs, and we cannot have brackets inside any key
    /// names.
    pub fn to_url(&self) -> Result<String> {
        let params = PinUrlParams::from(self.clone());
        let mut params_string = match serde_qs::to_string(&params) {
            Ok(p) => p,
            Err(e) => bail!("Failed to serialize query params. Err: {e}"),
        };
        params_string = params_string.replace('[', "%5B");
        params_string = params_string.replace(']', "%5D");
        Ok(format!("{URL_PREFIX}{params_string}"))
    }

    pub fn from_url(url: String) -> Result<Self> {
        let Some(stripped) = url.strip_prefix(URL_PREFIX) else {
            bail!("Url not prefixed with {URL_PREFIX}");
        };
        // Disable strict mode so nested values like `tags[0][0]=key` can still
        // be parsed, even if pasted into a browser that converted the brackets
        // into `tags%5B0%5D%5B0%5D=key`. Max depth is kept at default of 5.
        let config = serde_qs::Config::new(5, false);
        let params: PinUrlParams = match config.deserialize_str(stripped) {
            Ok(p) => p,
            Err(e) => bail!("Failed to parse URL query params. Err: {e}"),
        };
        Ok(Self::from(params))
    }

    /// Returns true if the pin passes all the active filters.
    pub fn passes_filters(&self, filters: &PinFilters) -> bool {
        filters
            .iter()
            .all(|(active, expr)| !active || expr.eval(self))
    }
}

/// Serializable url parameters for a Pin.
///
/// lng and lat are broken out so the url is shorter and easier to edit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)] // need bc empty vecs are dropped on ser, but req on deser
struct PinUrlParams {
    name: String,
    lng: f64,
    lat: f64,
    icon: String,
    lists: Vec<String>,
    tags: Vec<(String, String)>,
}

impl Default for PinUrlParams {
    fn default() -> Self {
        Pin::default().into()
    }
}

impl From<Pin> for PinUrlParams {
    fn from(p: Pin) -> Self {
        Self {
            name: p.name,
            lng: p.lnglat.lng,
            lat: p.lnglat.lat,
            icon: p.icon,
            lists: p.lists,
            tags: p.tags,
        }
    }
}

impl From<PinUrlParams> for Pin {
    fn from(p: PinUrlParams) -> Self {
        let mut out = Self::default();
        // use input validators
        out.set_name(p.name);
        let _ = out.set_icon(p.icon); // use default icon if invalid
        let _ = out.set_lnglat(LngLat {
            lng: p.lng,
            lat: p.lat,
        });
        out.set_lists(p.lists);
        out.set_tags(p.tags);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_url() {
        let mut pin = Pin {
            name: "mypin".into(),
            lnglat: LngLat {
                lng: -122.5,
                lat: 37.0,
            },
            ..Default::default()
        };

        // test with empty lists and tags
        let url = dbg!(pin.to_url().unwrap());
        let parsed = Pin::from_url(url).unwrap();
        assert_eq!(pin, parsed);

        // test with non-empty lists and tags
        pin.lists = vec!["lista".into(), "listb".into()];
        pin.tags = vec![
            ("taga".into(), "vala".into()),
            ("tagb".into(), "valb".into()),
        ];
        let url = dbg!(pin.to_url().unwrap());
        let parsed = Pin::from_url(url).unwrap();
        assert_eq!(pin, parsed);

        let url = "scoria://place?";
        let parsed = Pin::from_url(url.into()).unwrap();
        assert_eq!(Pin::default(), parsed);
    }
}

pub mod pin_predicate {
    use super::Pin;
    use crate::bool_expr::Predicate;
    use serde::{Deserialize, Serialize};
    use strum::{Display, EnumIter, EnumString};

    /// A predicate on a Pin using comparisons against a String, with
    /// comparisons being case insensitive (by converting the strings to be
    /// compared to lowercase)
    #[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
    pub struct PinStringPred {
        pub kind: PinStringPredKind,
        pub value: String,
    }

    #[derive(
        Clone,
        Debug,
        PartialEq,
        Default,
        Serialize,
        Deserialize,
        Display,
        EnumString,
        EnumIter,
    )]
    #[strum(serialize_all = "title_case")]
    pub enum PinStringPredKind {
        #[default]
        InList,
        NameContains,
        NameIs,
        IconContains,
        IconIs,
        HasTagKey,
        HasTagValue,
    }

    impl Predicate for PinStringPred {
        type Substitute = Pin;
        fn eval(&self, sub: &Self::Substitute) -> bool {
            let value = self.value.to_lowercase();
            match self.kind {
                PinStringPredKind::InList => {
                    sub.lists.iter().any(|l| l.to_lowercase() == value)
                }
                PinStringPredKind::NameContains => {
                    sub.name.to_lowercase().contains(&value)
                }
                PinStringPredKind::NameIs => sub.name.to_lowercase() == value,
                PinStringPredKind::IconContains => {
                    sub.icon.to_lowercase().contains(&value)
                }
                PinStringPredKind::IconIs => sub.icon.to_lowercase() == value,
                PinStringPredKind::HasTagKey => {
                    sub.tags.iter().any(|(k, _v)| k.to_lowercase() == value)
                }
                PinStringPredKind::HasTagValue => {
                    sub.tags.iter().any(|(_k, v)| v.to_lowercase() == value)
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::bool_expr::BoolExpr;
        #[test]
        fn test_pin_pred() {
            let mut pin = Pin::default();
            pin.lists.push("alist".into());
            pin.name = "pin name".into();
            pin.tags.push(("akey".into(), "aval".into()));

            assert!(BoolExpr::from(PinStringPred {
                kind: PinStringPredKind::InList,
                value: "alist".into(),
            })
            .eval(&pin));
            assert!(BoolExpr::from(PinStringPred {
                kind: PinStringPredKind::NameContains,
                value: "name".into(),
            })
            .eval(&pin));
            assert!(BoolExpr::from(PinStringPred {
                kind: PinStringPredKind::NameIs,
                value: "pin name".into(),
            })
            .eval(&pin));
            assert!(BoolExpr::Not(
                PinStringPred {
                    kind: PinStringPredKind::NameIs,
                    value: "not the name".into(),
                }
                .into()
            )
            .eval(&pin));
            assert!(BoolExpr::from(PinStringPred {
                kind: PinStringPredKind::IconContains,
                value: "⭐️".into(),
            })
            .eval(&pin));
            assert!(BoolExpr::from(PinStringPred {
                kind: PinStringPredKind::IconIs,
                value: "⭐️".into(),
            })
            .eval(&pin));
            assert!(BoolExpr::from(PinStringPred {
                kind: PinStringPredKind::HasTagKey,
                value: "akey".into(),
            })
            .eval(&pin));
            assert!(BoolExpr::from(PinStringPred {
                kind: PinStringPredKind::HasTagValue,
                value: "aval".into(),
            })
            .eval(&pin));
        }
    }
}
