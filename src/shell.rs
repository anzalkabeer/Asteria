use crate::css_parser::Stylesheet;
use crate::dom::Dom;
use crate::loader::{PageResources, ResourceLoader};

// ─── Browser Shell & Tab Architecture ─────────────────────────────
//
// Represents the browser application shell managing multiple active tabs,
// per-tab navigation history (back/forward/reload), page state, and
// user input events.

/// A unique 64-bit identifier for a browser tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(pub u64);

/// Per-tab navigation history stack.
#[derive(Debug, Clone, Default)]
pub struct NavigationHistory {
    /// Ordered list of visited URL strings
    pub stack: Vec<String>,
    /// Index of the current active URL in the history stack
    pub current_index: usize,
}

impl NavigationHistory {
    /// Create a new history stack initialized with a starting URL.
    pub fn new(initial_url: &str) -> Self {
        NavigationHistory {
            stack: vec![initial_url.to_string()],
            current_index: 0,
        }
    }

    /// Can the user navigate back in history?
    pub fn can_go_back(&self) -> bool {
        self.current_index > 0
    }

    /// Can the user navigate forward in history?
    pub fn can_go_forward(&self) -> bool {
        self.current_index + 1 < self.stack.len()
    }

    /// Navigate to a new URL, truncating any forward history.
    pub fn push_url(&mut self, url: &str) {
        if self.stack.is_empty() {
            self.stack.push(url.to_string());
            self.current_index = 0;
            return;
        }

        // Truncate any forward history past current index
        self.stack.truncate(self.current_index + 1);
        self.stack.push(url.to_string());
        self.current_index = self.stack.len() - 1;
    }

    /// Move back in history. Returns the target URL if successful.
    pub fn go_back(&mut self) -> Option<&str> {
        if self.can_go_back() {
            self.current_index -= 1;
            Some(&self.stack[self.current_index])
        } else {
            None
        }
    }

    /// Move forward in history. Returns the target URL if successful.
    pub fn go_forward(&mut self) -> Option<&str> {
        if self.can_go_forward() {
            self.current_index += 1;
            Some(&self.stack[self.current_index])
        } else {
            None
        }
    }

    /// Get the current active URL in history.
    pub fn current_url(&self) -> Option<&str> {
        self.stack.get(self.current_index).map(|s| s.as_str())
    }
}

/// A single browser tab containing its page state and parsed engine artifacts.
#[derive(Debug)]
pub struct Tab {
    pub id: TabId,
    pub url: String,
    pub title: String,
    pub history: NavigationHistory,
    pub page_resources: Option<PageResources>,
    pub dom: Option<Dom>,
    pub stylesheet: Option<Stylesheet>,
}

impl Tab {
    /// Create a new tab initialized with a starting URL.
    pub fn new(id: TabId, url: &str) -> Self {
        Tab {
            id,
            url: url.to_string(),
            title: if url == "<sample>" {
                "Sample Page".to_string()
            } else {
                url.to_string()
            },
            history: NavigationHistory::new(url),
            page_resources: None,
            dom: None,
            stylesheet: None,
        }
    }
}

/// User input and UI events dispatched to the browser shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellEvent {
    /// Navigate the active tab to a new URL
    NavigateTo(String),
    /// Open a new tab initialized with a URL
    NewTab(String),
    /// Close a tab by index
    CloseTab(usize),
    /// Switch active viewport tab by index
    SwitchTab(usize),
    /// Navigate active tab back in history
    GoBack,
    /// Navigate active tab forward in history
    GoForward,
    /// Reload current page in active tab
    Reload,
}

/// Multi-tab browser manager.
pub struct TabManager {
    next_tab_id: u64,
    pub tabs: Vec<Tab>,
    pub active_tab_index: usize,
    pub loader: ResourceLoader,
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TabManager {
    /// Create a new `TabManager` initialized with one default sample tab.
    pub fn new() -> Self {
        let initial_tab_id = TabId(1);
        let initial_tab = Tab::new(initial_tab_id, "<sample>");

        TabManager {
            next_tab_id: 2,
            tabs: vec![initial_tab],
            active_tab_index: 0,
            loader: ResourceLoader::new(),
        }
    }

    /// Open a new tab with the given URL and switch to it.
    /// Returns the new `TabId`.
    pub fn new_tab(&mut self, url: &str) -> TabId {
        let tab_id = TabId(self.next_tab_id);
        self.next_tab_id += 1;

        let mut tab = Tab::new(tab_id, url);
        let _ = self.load_tab_content(&mut tab);

        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
        tab_id
    }

    /// Close a tab by index. Re-adjusts active_tab_index if needed.
    /// Retains at least 1 tab (resets to default tab if last tab closed).
    pub fn close_tab(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() {
            return false;
        }

        self.tabs.remove(index);

        if self.tabs.is_empty() {
            // Keep at least one tab open
            let new_id = TabId(self.next_tab_id);
            self.next_tab_id += 1;
            let mut default_tab = Tab::new(new_id, "<sample>");
            let _ = self.load_tab_content(&mut default_tab);
            self.tabs.push(default_tab);
            self.active_tab_index = 0;
        } else if index < self.active_tab_index {
            self.active_tab_index -= 1;
        } else if self.active_tab_index >= self.tabs.len() {
            self.active_tab_index = self.tabs.len() - 1;
        }

        true
    }

    /// Switch active viewport tab by index.
    pub fn switch_tab(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            self.active_tab_index = index;
            true
        } else {
            false
        }
    }

    /// Get a reference to the active tab.
    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_tab_index]
    }

    /// Get a mutable reference to the active tab.
    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab_index]
    }

    /// Navigate active tab to a new URL, updating history stack.
    pub fn navigate(&mut self, url: &str) -> Result<(), String> {
        let index = self.active_tab_index;
        self.tabs[index].url = url.to_string();
        self.tabs[index].title = url.to_string();
        self.tabs[index].history.push_url(url);

        let mut tab = self.tabs.remove(index);
        let result = self.load_tab_content(&mut tab);
        self.tabs.insert(index, tab);

        result
    }

    /// Navigate active tab back in history.
    pub fn back(&mut self) -> Result<bool, String> {
        let index = self.active_tab_index;
        if let Some(prev_url) = self.tabs[index].history.go_back().map(|s| s.to_string()) {
            self.tabs[index].url = prev_url.clone();
            self.tabs[index].title = prev_url;
            let mut tab = self.tabs.remove(index);
            let result = self.load_tab_content(&mut tab);
            self.tabs.insert(index, tab);
            result.map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Navigate active tab forward in history.
    pub fn forward(&mut self) -> Result<bool, String> {
        let index = self.active_tab_index;
        if let Some(next_url) = self.tabs[index].history.go_forward().map(|s| s.to_string()) {
            self.tabs[index].url = next_url.clone();
            self.tabs[index].title = next_url;
            let mut tab = self.tabs.remove(index);
            let result = self.load_tab_content(&mut tab);
            self.tabs.insert(index, tab);
            result.map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Reload the active tab's current URL.
    pub fn reload(&mut self) -> Result<(), String> {
        let index = self.active_tab_index;
        let mut tab = self.tabs.remove(index);
        let result = self.load_tab_content(&mut tab);
        self.tabs.insert(index, tab);
        result
    }

    /// Dispatch a shell event to update tab manager state.
    pub fn handle_event(&mut self, event: ShellEvent) -> Result<(), String> {
        match event {
            ShellEvent::NavigateTo(url) => self.navigate(&url),
            ShellEvent::NewTab(url) => {
                self.new_tab(&url);
                Ok(())
            }
            ShellEvent::CloseTab(index) => {
                self.close_tab(index);
                Ok(())
            }
            ShellEvent::SwitchTab(index) => {
                self.switch_tab(index);
                Ok(())
            }
            ShellEvent::GoBack => self.back().map(|_| ()),
            ShellEvent::GoForward => self.forward().map(|_| ()),
            ShellEvent::Reload => self.reload(),
        }
    }

    /// Load resources and parse engine structures for a tab.
    /// Returns Ok(()) on success or Err(message) on failure,
    /// ensuring stale artifacts are cleared if loading fails.
    fn load_tab_content(&mut self, tab: &mut Tab) -> Result<(), String> {
        // Clear any stale artifacts first
        tab.page_resources = None;
        tab.dom = None;
        tab.stylesheet = None;

        if tab.url == "<sample>" {
            let sample_html =
                "<!DOCTYPE html><html><body><h1>Asteria Shell</h1><p>Sample Tab</p></body></html>";
            let page = self.loader.load_html_string(sample_html, "<sample>");

            let mut tokenizer = crate::tokenizer::Tokenizer::new(&page.html.bytes);
            let tokens = tokenizer.tokenize();
            let parser = crate::parser::Parser::new(&tokens, &page.html.bytes);
            let dom = parser.parse();

            let mut css_bytes = Vec::new();
            for res in &page.stylesheets {
                css_bytes.extend_from_slice(&res.bytes);
            }
            let stylesheet = Stylesheet::parse(&css_bytes);

            tab.page_resources = Some(page);
            tab.dom = Some(dom);
            tab.stylesheet = Some(stylesheet);
            Ok(())
        } else {
            let page = self
                .loader
                .load_resource(&tab.url)
                .map_err(|e| e.to_string())?;

            let mut tokenizer = crate::tokenizer::Tokenizer::new(&page.html.bytes);
            let tokens = tokenizer.tokenize();
            let parser = crate::parser::Parser::new(&tokens, &page.html.bytes);
            let dom = parser.parse();

            let mut css_bytes = Vec::new();
            for res in &page.stylesheets {
                css_bytes.extend_from_slice(&res.bytes);
            }
            let stylesheet = Stylesheet::parse(&css_bytes);

            tab.page_resources = Some(page);
            tab.dom = Some(dom);
            tab.stylesheet = Some(stylesheet);
            Ok(())
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_history() {
        let mut history = NavigationHistory::new("page1.html");
        assert_eq!(history.current_url(), Some("page1.html"));
        assert!(!history.can_go_back());
        assert!(!history.can_go_forward());

        history.push_url("page2.html");
        assert_eq!(history.current_url(), Some("page2.html"));
        assert!(history.can_go_back());
        assert!(!history.can_go_forward());

        history.push_url("page3.html");
        assert_eq!(history.current_url(), Some("page3.html"));

        // Go back
        assert_eq!(history.go_back(), Some("page2.html"));
        assert_eq!(history.current_url(), Some("page2.html"));
        assert!(history.can_go_forward());

        // Go back again
        assert_eq!(history.go_back(), Some("page1.html"));
        assert_eq!(history.current_url(), Some("page1.html"));
        assert!(!history.can_go_back());

        // Go forward
        assert_eq!(history.go_forward(), Some("page2.html"));
        assert_eq!(history.current_url(), Some("page2.html"));

        // Push new URL truncates forward history (page3 disappears)
        history.push_url("page4.html");
        assert_eq!(history.current_url(), Some("page4.html"));
        assert!(!history.can_go_forward());
        assert_eq!(
            history.stack,
            vec!["page1.html", "page2.html", "page4.html"]
        );
    }

    #[test]
    fn test_tab_manager_tabs_and_switching() {
        let mut manager = TabManager::new();
        assert_eq!(manager.tabs.len(), 1);
        assert_eq!(manager.active_tab_index, 0);

        let id2 = manager.new_tab("<sample>");
        assert_eq!(manager.tabs.len(), 2);
        assert_eq!(manager.active_tab_index, 1);
        assert_eq!(manager.active_tab().id, id2);

        // Switch to first tab
        assert!(manager.switch_tab(0));
        assert_eq!(manager.active_tab_index, 0);

        // Close second tab
        assert!(manager.close_tab(1));
        assert_eq!(manager.tabs.len(), 1);

        // Close last remaining tab creates a new default tab
        assert!(manager.close_tab(0));
        assert_eq!(manager.tabs.len(), 1);
    }

    #[test]
    fn test_tab_manager_navigation_events() {
        let mut manager = TabManager::new();
        manager
            .handle_event(ShellEvent::NavigateTo("<sample>".to_string()))
            .unwrap();

        assert_eq!(manager.active_tab().url, "<sample>");
        assert!(manager.active_tab().dom.is_some());
        assert!(manager.active_tab().stylesheet.is_some());
    }

    #[test]
    fn test_close_tab_before_active() {
        let mut manager = TabManager::new(); // tab 0: <sample>
        manager.new_tab("<sample>"); // tab 1
        let id3 = manager.new_tab("<sample>"); // tab 2 (active)

        assert_eq!(manager.active_tab_index, 2);
        assert_eq!(manager.active_tab().id, id3);

        // Close tab 0 (which is before active_tab_index)
        assert!(manager.close_tab(0));

        // Active tab should now be index 1, pointing to the exact same tab id3
        assert_eq!(manager.active_tab_index, 1);
        assert_eq!(manager.active_tab().id, id3);

        // Close tab 0 again (which is before active_tab_index)
        assert!(manager.close_tab(0));
        assert_eq!(manager.active_tab_index, 0);
        assert_eq!(manager.active_tab().id, id3);
    }

    #[test]
    fn test_navigate_failure_clears_artifacts() {
        let mut manager = TabManager::new();
        let _ = manager.navigate("<sample>");
        assert!(manager.active_tab().dom.is_some());

        // Navigate to non-existent file
        let result = manager.navigate("non_existent_file_xyz_9999.html");
        assert!(result.is_err());

        // Artifacts should be cleared, not stale
        assert!(manager.active_tab().page_resources.is_none());
        assert!(manager.active_tab().dom.is_none());
        assert!(manager.active_tab().stylesheet.is_none());
    }
}
