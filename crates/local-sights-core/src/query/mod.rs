//! The Logs Insights compatible query of the Log Analytics screen
//! (261017-insights-query).
//!
//! U1 (the walking skeleton) read a query made only of `filter` commands
//! and evaluated it on the events the query screen fetched. U2 reads the
//! rest of the language ([`parser`], into a [`plan::QueryPlan`] of `fields`,
//! `filter`, `sort` and `limit` in the order written), reads JSON fields of
//! the messages ([`fields`]), compares values of different types and sorts in
//! the documentation's natural order ([`natural`]) and applies the commands
//! one after another ([`evaluator`]). The date texts compared with
//! `@timestamp` are turned into milliseconds in the chosen time zone
//! ([`instant`]). The events live in the query screen's own holdings
//! ([`store`]), apart from the Log Management screen's EventTimeline
//! (261017-insights-query U1:BR1.7, BR2.1).
//!
//! Nothing here calls an AWS API by itself: the fetch is the existing
//! FetchCoordinator flow (DescribeLogStreams and GetLogEvents only, U1:BR2.2).

pub mod evaluator;
pub mod fields;
pub mod instant;
pub mod natural;
pub mod parser;
pub mod plan;
pub mod runner;
pub mod store;
