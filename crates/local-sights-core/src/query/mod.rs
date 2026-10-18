//! The Logs Insights compatible query of the Log Analytics screen
//! (261017-insights-query).
//!
//! U1 (the walking skeleton) reads a query made only of `filter` commands
//! ([`parser`], into a [`plan::QueryPlan`]), turns the date texts compared
//! with `@timestamp` into milliseconds in the chosen time zone, reads the
//! fixed fields of an event and evaluates the plan on the events the query
//! screen fetched. Those events live in the query screen's own holdings,
//! apart from the Log Management screen's EventTimeline (261017-insights-query
//! U1:BR1.7, BR2.1).
//!
//! Nothing here calls an AWS API by itself: the fetch is the existing
//! FetchCoordinator flow (DescribeLogStreams and GetLogEvents only, U1:BR2.2).

pub mod evaluator;
pub mod fields;
pub mod instant;
pub mod parser;
pub mod plan;
pub mod runner;
pub mod store;
