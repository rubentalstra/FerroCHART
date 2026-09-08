// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1
//! The browser session a journey drives, and the waits it is built from.
//!
//! Every failure names the screen it happened on and the step that was
//! waiting, because a selector that matched nothing says neither by itself.

use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use thirtyfour::LoggingPrefsLogLevel;
use thirtyfour::prelude::*;

/// Names the renderer under test, as a URL without a trailing slash.
pub(crate) const BASE_URL_ENV: &str = "FERROCHART_UI_E2E_BASE_URL";

/// Names the `WebDriver` endpoint the browser is driven through.
pub(crate) const WEBDRIVER_ENV: &str = "FERROCHART_UI_E2E_WEBDRIVER";

/// The `WebDriver` endpoint used when [`WEBDRIVER_ENV`] is unset.
const DEFAULT_WEBDRIVER: &str = "http://127.0.0.1:4444";

/// Names the directory a failed wait writes its evidence into.
pub(crate) const FAILURES_ENV: &str = "FERROCHART_UI_E2E_FAILURES";

/// Where the evidence goes when nothing names a directory.
const DEFAULT_FAILURES: &str = "target/ui-e2e-failures";

/// How long a wait keeps trying before the journey fails.
const WAIT: Duration = Duration::from_secs(30);

/// How often a wait re-reads the page while it waits.
const POLL: Duration = Duration::from_millis(100);

/// The level chromedriver reports a `console.error` and an uncaught panic at.
///
/// The renderer installs `console_error_panic_hook`, so a panic in the
/// WebAssembly bundle arrives here rather than vanishing.
const SEVERE: &str = "SEVERE";

/// How much of a failing page's markup the report carries.
///
/// Enough to see which sections rendered, short of burying the assertion. The
/// whole document goes to the evidence file beside it.
const MARKUP_IN_A_FAILURE: usize = 20_000;

/// How many words of a step's description name its evidence file.
const SLUG_WORDS: usize = 8;

/// The identifier of the shell's scrolling content region.
///
/// The shell is a fixed-height frame whose content region scrolls inside it,
/// so the document is always exactly as tall as the viewport and the region
/// is the only thing that knows how tall a screen really is.
const CONTENT_REGION: &str = "main";

/// The flags the browser is started with.
///
/// `--headless=new` is Chrome's current headless mode; the other two are what
/// a browser inside a container needs, because the kernel sandbox and the
/// default 64 MB `/dev/shm` are both unavailable there
/// (<https://developer.chrome.com/docs/chromium/headless>).
const CHROME_ARGS: [&str; 4] = [
    "--headless=new",
    "--no-sandbox",
    "--disable-dev-shm-usage",
    "--window-size=1440,900",
];

/// Reads the viewport height and how much of the screen falls outside it.
///
/// `WebDriver` reports the rendered box of an element and nothing about what
/// overflows it (<https://www.w3.org/TR/webdriver2/#get-element-rect>), so how
/// far a screen runs past the window is only readable by asking the document.
/// Both places it can run past are read: the shell's outer box is
/// `min-h-screen`, so a tall screen grows the document, and its content region
/// carries `overflow-auto`, so a short window can instead leave the region
/// itself scrolling. The script reads three numbers and changes nothing.
const MEASURE: &str = "const region = document.getElementById(arguments[0]);
return [
  window.innerHeight,
  document.documentElement.scrollHeight,
  region ? region.scrollHeight - region.clientHeight : 0,
];";

/// Collects the words on a screen that name the openEHR model.
///
/// No specification governs this: our own design. A person building a form is
/// not required to know the openEHR Reference Model, so a class name in the
/// text they read is a defect. Two surfaces are deliberate and exempt: a path
/// inside a monospaced element, which is what stays available for whoever
/// wants it, and the bracketed code an unlabelled field carries after what it
/// collects, which is what tells two siblings apart. #178 bans a BARE class
/// name or code, and those two are not bare.
const JARGON: &str = "const pattern = /\\b(DV_[A-Z][A-Z_]*|CODE_PHRASE|ITEM_[A-Z]+|ADMIN_ENTRY|POINT_EVENT|INTERVAL_EVENT|CLUSTER|ELEMENT|OBSERVATION|EVALUATION|INSTRUCTION|SECTION|HISTORY|at[0-9]{4,})\\b/;
const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
const found = [];
let node;
while ((node = walker.nextNode())) {
  const parent = node.parentElement;
  if (!parent || parent.closest('code, pre, .font-mono')) { continue; }
  const text = node.textContent.trim();
  if (!text) { continue; }
  const said = text.replace(/\\s*\\([^()]*\\)\\s*$/, '');
  const hit = said.match(pattern);
  if (hit) { found.push(hit[0] + ' in \"' + text.slice(0, 120) + '\"'); }
}
return found.slice(0, 20);";
/// The renderer under test, or `None` when nothing names one.
pub(crate) fn renderer() -> Option<String> {
    match std::env::var(BASE_URL_ENV) {
        Ok(base) => Some(base.trim_end_matches('/').to_owned()),
        Err(_absent) => {
            println!(
                "skipped: {BASE_URL_ENV} is unset, so there is no served renderer to drive. \
                 scripts/ui-e2e.sh sets it, and the ui-e2e CI job always runs that script."
            );
            None
        }
    }
}

/// A browser session on the configured `WebDriver` endpoint.
pub(crate) async fn session() -> WebDriver {
    let endpoint =
        std::env::var(WEBDRIVER_ENV).unwrap_or_else(|_absent| DEFAULT_WEBDRIVER.to_owned());
    let mut capabilities = DesiredCapabilities::chrome();
    for argument in CHROME_ARGS {
        capabilities
            .add_arg(argument)
            .expect("chrome accepts a command-line argument");
    }
    // chromedriver keeps a browser log only for a session that asked for one
    // when it was created, and that log is where an uncaught panic lands.
    capabilities
        .set_browser_log_level(LoggingPrefsLogLevel::All)
        .expect("chrome accepts a logging preference");
    WebDriver::new(&endpoint, capabilities)
        .await
        .unwrap_or_else(|error| panic!("no browser session at {endpoint}: {error}"))
}

/// `what` as a file name: lowercase words joined by hyphens.
fn slug(what: &str) -> String {
    let letters: String = what
        .chars()
        .map(|letter| {
            if letter.is_ascii_alphanumeric() {
                letter.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = letters
        .split('-')
        .filter(|word| !word.is_empty())
        .take(SLUG_WORDS)
        .collect::<Vec<&str>>()
        .join("-");
    if trimmed.is_empty() {
        "failure".to_owned()
    } else {
        trimmed
    }
}

/// One browser session, on one screen of the renderer.
#[derive(Debug)]
pub(crate) struct Page {
    /// The session every step drives.
    driver: WebDriver,
    /// The screen the session is on, which names every failure.
    screen: String,
}

impl Page {
    /// Opens `address` and returns the page over it, named by `screen`.
    pub(crate) async fn open(driver: WebDriver, screen: &str, address: &str) -> Self {
        let page = Self {
            driver,
            screen: screen.to_owned(),
        };
        page.go(screen, address).await;
        page
    }

    /// Renames the page and loads `address` in the same session.
    ///
    /// A capture walks several screens in one session, so the name travels
    /// with the address and a failure still says which screen it was on.
    pub(crate) async fn go(&self, screen: &str, address: &str) {
        self.driver.goto(address).await.unwrap_or_else(|error| {
            panic!("{screen}: the browser could not open {address}: {error}")
        });
    }

    /// The screen this page is on.
    pub(crate) fn screen(&self) -> &str {
        &self.screen
    }

    /// Every openEHR class name and node code in the text this screen shows.
    ///
    /// Empty is the passing answer. What sits inside a monospaced element is
    /// the deliberate machine-readable surface and is not counted.
    pub(crate) async fn jargon(&self) -> Vec<String> {
        let read = self
            .driver
            .execute(JARGON, Vec::new())
            .await
            .unwrap_or_else(|error| panic!("{}: reading the text drawn: {error}", self.screen));
        read.convert().unwrap_or_else(|error| {
            panic!("{}: the jargon walk answered oddly: {error}", self.screen)
        })
    }

    /// Puts `screen` on every failure this page reports from here on.
    pub(crate) fn now_on(&mut self, screen: &str) {
        screen.clone_into(&mut self.screen);
    }

    /// The first element matching `selector`, once the bundle has drawn it.
    ///
    /// The wait is the library's own poller over the element condition, so
    /// nothing here sleeps for a fixed time and hopes.
    pub(crate) async fn element(&self, selector: By, what: &str) -> WebElement {
        match self.driver.query(selector).wait(WAIT, POLL).first().await {
            Ok(element) => element,
            Err(error) => panic!("{}", self.failure(what, &error.to_string()).await),
        }
    }

    /// Waits until at least `least` elements match `selector`.
    ///
    /// A screen draws its parts one render at a time, so a count taken the
    /// moment the first one appears is a count of a page still filling in.
    pub(crate) async fn at_least(&self, selector: By, least: usize, what: &str) -> usize {
        let deadline = Instant::now() + WAIT;
        loop {
            let found = self.count(selector.clone()).await;
            if found >= least {
                return found;
            }
            if Instant::now() >= deadline {
                let reason = format!("{found} of them are on the page, and {least} were wanted");
                panic!("{}", self.failure(what, &reason).await);
            }
            tokio::time::sleep(POLL).await;
        }
    }

    /// How many elements match `selector` right now.
    pub(crate) async fn count(&self, selector: By) -> usize {
        self.driver
            .find_all(selector)
            .await
            .map(|found| found.len())
            .unwrap_or_default()
    }

    /// Fails when anything matches `selector`.
    ///
    /// The caller anchors on something the same render draws before calling
    /// this, so an empty count is the screen having decided rather than the
    /// screen not having drawn yet.
    pub(crate) async fn none(&self, selector: By, what: &str) {
        let found = self.count(selector).await;
        if found > 0 {
            let reason = format!("{found} of them are on the page, and none was wanted");
            panic!("{}", self.failure(what, &reason).await);
        }
    }

    /// The text of every element matching `selector`, in document order.
    pub(crate) async fn texts(&self, selector: By) -> Vec<String> {
        let Ok(found) = self.driver.find_all(selector).await else {
            return Vec::new();
        };
        let mut read = Vec::with_capacity(found.len());
        for element in found {
            read.push(element.text().await.unwrap_or_default());
        }
        read
    }

    /// The value of `attribute` on the first element matching `selector`.
    pub(crate) async fn attribute(&self, selector: By, attribute: &str, what: &str) -> String {
        let element = self.element(selector, what).await;
        match element.attr(attribute).await {
            Ok(value) => value.unwrap_or_default(),
            Err(error) => panic!("{}", self.failure(what, &error.to_string()).await),
        }
    }

    /// Waits until `selector` carries one of `wanted` in its `class`, and
    /// returns the one it carries.
    ///
    /// The shell writes the ground onto the document element from an effect
    /// after it mounts, so the class read the moment a page loads is the class
    /// of a document the bundle has not reached yet.
    pub(crate) async fn class_becoming(&self, selector: By, wanted: &[&str], what: &str) -> String {
        let deadline = Instant::now() + WAIT;
        loop {
            let carried = self.attribute(selector.clone(), "class", what).await;
            if let Some(found) = carried
                .split_whitespace()
                .find(|name| wanted.contains(name))
            {
                return (*found).to_owned();
            }
            if Instant::now() >= deadline {
                let reason = format!("the class is `{carried}`, and one of {wanted:?} was wanted");
                panic!("{}", self.failure(what, &reason).await);
            }
            tokio::time::sleep(POLL).await;
        }
    }

    /// Clicks the first element matching `selector`, once it is there.
    pub(crate) async fn click(&self, selector: By, what: &str) {
        let element = self.element(selector, what).await;
        if let Err(error) = element.click().await {
            panic!("{}", self.failure(what, &error.to_string()).await);
        }
    }

    /// Everything the browser logged as severe since the last read.
    ///
    /// Each read drains the buffer, so it is read only where it is reported.
    /// Nothing is passed over: a failed request for an asset or a form
    /// definition is exactly the defect a rendering test exists to catch.
    async fn console_errors(&self) -> Vec<String> {
        match self.driver.browser_log().await {
            Ok(entries) => entries
                .iter()
                .filter(|entry| entry.level == SEVERE)
                .map(|entry| {
                    let source = entry.source.as_deref().unwrap_or("unattributed");
                    format!("[{source}] {}", entry.message)
                })
                .collect(),
            Err(error) => vec![format!("the browser log could not be read: {error}")],
        }
    }

    /// Fails the journey when the browser logged anything severe.
    pub(crate) async fn quiet(&self) {
        let logged = self.console_errors().await;
        assert!(
            logged.is_empty(),
            "{}: the browser logged {} severe entries, and a drawn screen that logs one is a defect:\n  {}",
            self.screen,
            logged.len(),
            logged.join("\n  ")
        );
    }

    /// Sets the browser window, and with it the viewport a shot is taken of.
    async fn resize(&self, width: u32, height: u32) {
        self.driver
            .set_window_rect(0, 0, width, height)
            .await
            .unwrap_or_else(|error| {
                panic!("{}: resizing to {width}x{height}: {error}", self.screen)
            });
    }

    /// The viewport height, the document's full height, and what the content
    /// region hides inside its own scroll.
    async fn measure(&self) -> WebDriverResult<(u32, u32, u32)> {
        let read = self
            .driver
            .execute(MEASURE, vec![CONTENT_REGION.into()])
            .await?;
        let numbers: Vec<i64> = read.convert()?;
        let at = |index: usize| -> u32 {
            numbers
                .get(index)
                .copied()
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or_default()
        };
        Ok((at(0), at(1), at(2)))
    }

    /// Grows the window until the whole screen is inside the viewport, then
    /// writes a PNG of it to `path`.
    ///
    /// A screenshot is the viewport
    /// (<https://www.w3.org/TR/webdriver2/#take-screenshot>), so a screen
    /// taller than the window is cut off at whatever the window happened to
    /// be. The window starts at the floor, the measurement says how far the
    /// screen runs past it, and the window is then set to fit, plus whatever
    /// frame the browser puts around a viewport, which the driver's rectangle
    /// includes and the shot does not.
    pub(crate) async fn shoot(
        &self,
        path: &Path,
        width: u32,
        bounds: (u32, u32),
    ) -> WebDriverResult<()> {
        self.resize(width, bounds.0).await;
        let (viewport, document, hidden) = self.measure().await?;
        let overflow = document.saturating_sub(viewport).max(hidden);
        let wanted = viewport.saturating_add(overflow).clamp(bounds.0, bounds.1);
        let frame = bounds.0.saturating_sub(viewport);
        self.resize(width, wanted.saturating_add(frame)).await;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("creating {}: {error}", parent.display()));
        }
        self.driver
            .screenshot(path)
            .await
            .unwrap_or_else(|error| panic!("writing {}: {error}", path.display()));
        // The document height is printed beside the file, because how far a
        // form runs is the measurement #190 is judged by and there is nowhere
        // else it can be read: an image is cropped to the capture bounds, so
        // two forms of very different length photograph the same size.
        println!("captured {} ({document} px tall)", path.display());
        Ok(())
    }

    /// The message for a wait that ran out, with what the browser logged.
    ///
    /// A boot failure shows up as an element that never appears, so the
    /// console, the address and the markup all go into the message: without
    /// them the report says only that a selector did not match.
    async fn failure(&self, what: &str, reason: &str) -> String {
        let console = self.console_errors().await;
        let logged = if console.is_empty() {
            "the browser logged nothing severe".to_owned()
        } else {
            format!("the browser logged:\n  {}", console.join("\n  "))
        };
        let address = self.driver.current_url().await.map_or_else(
            |error| format!("(the address could not be read: {error})"),
            |url| url.to_string(),
        );
        let source = self.driver.source().await;
        let markup = match source {
            Ok(ref text) => {
                let trimmed: String = text.chars().take(MARKUP_IN_A_FAILURE).collect();
                format!("the page held:\n{trimmed}")
            }
            Err(ref error) => format!("(the markup could not be read: {error})"),
        };
        let evidence = self.evidence(what, source.ok().as_deref()).await;
        format!(
            "{}: waiting for {what} failed: {reason}\nthe address was {address}\n\
             {logged}\n{evidence}\n{markup}",
            self.screen
        )
    }

    /// Writes a screenshot and the whole document to the failure directory.
    ///
    /// The `ui-e2e` CI job uploads that directory when a run fails, so a
    /// failure that reproduces only on the runner is looked at rather than
    /// re-run. Nothing here may raise: a browser that cannot answer a
    /// screenshot request is already the failure being reported, and a second
    /// panic on the way out would hide the first.
    async fn evidence(&self, what: &str, source: Option<&str>) -> String {
        let dir = std::env::var(FAILURES_ENV)
            .map_or_else(|_absent| PathBuf::from(DEFAULT_FAILURES), PathBuf::from);
        if let Err(error) = std::fs::create_dir_all(&dir) {
            return format!("(no evidence written: creating {}: {error})", dir.display());
        }
        // nextest gives each test its own process, and two journeys can fail
        // on the same step, so the pid keeps one from overwriting the other.
        let stem = format!(
            "{}-{}-{}",
            slug(&self.screen),
            slug(what),
            std::process::id()
        );
        let shot = dir.join(format!("{stem}.png"));
        let page = dir.join(format!("{stem}.html"));
        let mut written = Vec::new();
        match self.driver.screenshot(&shot).await {
            Ok(()) => written.push(format!("a screenshot in {}", shot.display())),
            Err(error) => written.push(format!("(no screenshot: {error})")),
        }
        match source {
            Some(text) => match std::fs::write(&page, text) {
                Ok(()) => written.push(format!("the whole document in {}", page.display())),
                Err(error) => {
                    written.push(format!(
                        "(no document: writing {}: {error})",
                        page.display()
                    ));
                }
            },
            None => written.push("(no document: the markup could not be read)".to_owned()),
        }
        format!("the failure left {}", written.join(", and "))
    }
}

#[cfg(test)]
mod tests {
    use super::slug;

    #[test]
    fn a_steps_description_names_a_file_a_filesystem_accepts() {
        assert_eq!(
            slug("a control a clinician can type into"),
            "a-control-a-clinician-can-type-into"
        );
        assert_eq!(
            slug("/ui/forms/Family history summary item R2"),
            "ui-forms-family-history-summary-item-r2",
            "a description carrying an address leaves no separator in the name"
        );
        assert_eq!(
            slug("a control a clinician can enter a value into, on a repeatable group"),
            "a-control-a-clinician-can-enter-a-value",
            "a long description is cut to a name a listing can still read"
        );
        assert_eq!(slug("..."), "failure", "a name is never empty");
    }
}
