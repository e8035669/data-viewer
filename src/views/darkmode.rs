use anyhow;
use dioxus::{logger::tracing, prelude::*};
use std::{fmt, str::FromStr};

#[derive(Default)]
pub enum Theme {
    #[default]
    Auto,
    Light,
    Dark,
}

impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Theme::Auto => write!(f, "auto"),
            Theme::Light => write!(f, "light"),
            Theme::Dark => write!(f, "dark"),
        }
    }
}

#[derive(Debug)]
pub struct ParseError;

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseError")
    }
}

impl std::error::Error for ParseError {}

impl FromStr for Theme {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(Theme::Auto),
            "light" => Ok(Theme::Light),
            "dark" => Ok(Theme::Dark),
            _ => Err(ParseError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeProvider;

impl ThemeProvider {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn set_theme(&self, theme: Theme) {
        let prog = format!(r#"window.themeProvider.setTheme("{theme}")"#);
        let ret = document::eval(&prog).await;
        tracing::info!("{:?}", ret);
    }

    pub async fn get_theme(&self) -> anyhow::Result<Theme> {
        let prog = format!("return window.themeProvider.getTheme()");
        let ret = document::eval(&prog).await?;
        let theme = Theme::from_str(&ret.to_string())?;
        Ok(theme)
    }
}

pub fn use_theme_context() -> ThemeProvider {
    use_context()
}
