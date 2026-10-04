//! End-of-pages decision for GetLogEvents (BR3.2).
//!
//! GetLogEvents signals the last page by returning the same forward token
//! that was sent. An empty or short page is *not* the end. A response with
//! no token at all is treated as the end defensively, because there is
//! nothing left to follow.

/// What to do after a response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageDecision {
    /// Send the received token and read the next page.
    Continue,
    /// No more pages.
    Finished,
}

/// Decides whether paging continues, given the token that was sent
/// (`None` on the first call) and the token that came back.
pub fn decide(sent: Option<&str>, received: Option<&str>) -> PageDecision {
    match (sent, received) {
        (_, None) => PageDecision::Finished,
        (Some(sent), Some(received)) if sent == received => PageDecision::Finished,
        _ => PageDecision::Continue,
    }
}

/// Tracks the paging state of one stream: the token to send next and the
/// number of responses received (`pageCount`).
#[derive(Debug, Clone, Default)]
pub struct PageCursor {
    sent_token: Option<String>,
    page_count: u32,
    finished: bool,
}

impl PageCursor {
    /// A cursor before the first call.
    pub fn new() -> Self {
        Self::default()
    }

    /// Token to send with the next call (`None` for the first call).
    pub fn token_to_send(&self) -> Option<&str> {
        self.sent_token.as_deref()
    }

    /// Records one response, counts it, and decides what happens next.
    pub fn record_response(&mut self, received: Option<&str>) -> PageDecision {
        self.page_count = self.page_count.saturating_add(1);
        let decision = decide(self.sent_token.as_deref(), received);
        match decision {
            PageDecision::Continue => self.sent_token = received.map(str::to_string),
            PageDecision::Finished => self.finished = true,
        }
        decision
    }

    /// Number of responses received so far, including the final one that
    /// repeated the sent token.
    pub fn page_count(&self) -> u32 {
        self.page_count
    }

    /// Whether the last page has been reached.
    pub fn is_finished(&self) -> bool {
        self.finished
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_response_with_token_continues() {
        assert_eq!(decide(None, Some("f/1")), PageDecision::Continue);
    }

    #[test]
    fn first_response_without_token_finishes() {
        assert_eq!(decide(None, None), PageDecision::Finished);
    }

    #[test]
    fn later_response_with_same_token_finishes() {
        assert_eq!(decide(Some("f/2"), Some("f/2")), PageDecision::Finished);
    }

    #[test]
    fn later_response_with_different_token_continues() {
        assert_eq!(decide(Some("f/2"), Some("f/3")), PageDecision::Continue);
    }

    #[test]
    fn later_response_without_token_finishes() {
        assert_eq!(decide(Some("f/2"), None), PageDecision::Finished);
    }

    #[test]
    fn cursor_follows_tokens_and_counts_the_final_same_token_response() {
        let mut cursor = PageCursor::new();
        assert_eq!(cursor.token_to_send(), None);
        assert_eq!(cursor.page_count(), 0);

        // Page 1 (may be empty: an empty page is never the end).
        assert_eq!(cursor.record_response(Some("f/1")), PageDecision::Continue);
        assert_eq!(cursor.token_to_send(), Some("f/1"));
        // Page 2.
        assert_eq!(cursor.record_response(Some("f/2")), PageDecision::Continue);
        assert_eq!(cursor.token_to_send(), Some("f/2"));
        // Page 3 repeats the sent token: end of pages, still counted.
        assert_eq!(cursor.record_response(Some("f/2")), PageDecision::Finished);
        assert_eq!(cursor.page_count(), 3);
        assert!(cursor.is_finished());
    }

    #[test]
    fn cursor_finishes_after_single_tokenless_response() {
        let mut cursor = PageCursor::new();
        assert_eq!(cursor.record_response(None), PageDecision::Finished);
        assert_eq!(cursor.page_count(), 1);
        assert!(cursor.is_finished());
    }
}
