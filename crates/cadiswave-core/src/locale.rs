//! Stable language preferences independent of the audio runtime locale.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageChoice {
    #[default]
    #[serde(rename = "system")]
    System,
    #[serde(rename = "en")]
    English,
    #[serde(rename = "id")]
    Indonesian,
}
impl LanguageChoice {
    pub fn resolve(self, system_locale: &str) -> &'static str {
        match self {
            Self::English => "en",
            Self::Indonesian => "id",
            Self::System => {
                if system_locale
                    .split(['_', '-', '.', ':'])
                    .next()
                    .is_some_and(|language| language.eq_ignore_ascii_case("id"))
                {
                    "id"
                } else {
                    "en"
                }
            }
        }
    }
}
