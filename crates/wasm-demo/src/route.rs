#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Home,
    Components,
    Component(String),
    NotFound(String),
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
impl Route {
    pub fn path(&self) -> String {
        match self {
            Self::Home => "/".to_owned(),
            Self::Components => "/components".to_owned(),
            Self::Component(slug) => format!("/docs/components/{slug}"),
            Self::NotFound(path) => path.clone(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn read_initial_route() -> Route {
    let Some(window) = web_sys::window() else {
        return Route::Home;
    };
    let Ok(pathname) = window.location().pathname() else {
        return Route::Home;
    };
    parse_path(&pathname)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_initial_route() -> Route {
    Route::Home
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn parse_path(path: &str) -> Route {
    let normalized = normalize_path(path);

    if normalized == "/" {
        return Route::Home;
    }
    if normalized == "/components" {
        return Route::Components;
    }
    if let Some(slug) = normalized.strip_prefix("/docs/components/") {
        return Route::Component(slug.to_owned());
    }

    Route::NotFound(normalized)
}

#[cfg(target_arch = "wasm32")]
pub fn sync_url(route: &Route) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let wanted = route.path();
    let current = window.location().pathname().unwrap_or_default();
    if normalize_path(&current) == normalize_path(&wanted) {
        return;
    }

    if let Ok(history) = window.history() {
        let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&wanted));
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn sync_url(_route: &Route) {}

#[cfg(target_arch = "wasm32")]
pub fn current_window_width() -> Option<f32> {
    let window = web_sys::window()?;
    let width = window.inner_width().ok()?.as_f64()?;
    Some(width as f32)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn current_window_width() -> Option<f32> {
    None
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn normalize_path(path: &str) -> String {
    let mut normalized = path.trim().to_owned();
    if normalized.is_empty() {
        normalized.push('/');
    }
    if !normalized.starts_with('/') {
        normalized.insert(0, '/');
    }
    if normalized.len() > 1 {
        normalized = normalized.trim_end_matches('/').to_owned();
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_path_maps_root_and_components() {
        assert_eq!(parse_path("/"), Route::Home);
        assert_eq!(parse_path("/components"), Route::Components);
    }

    #[test]
    fn parse_path_normalizes_trailing_slashes() {
        assert_eq!(
            parse_path("/docs/components/marquee/"),
            Route::Component("marquee".to_owned())
        );
    }

    #[test]
    fn parse_path_preserves_unknown_routes() {
        assert_eq!(
            parse_path("/missing"),
            Route::NotFound("/missing".to_owned())
        );
    }
}
