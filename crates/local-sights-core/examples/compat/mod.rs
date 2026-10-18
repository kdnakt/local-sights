//! The parts of `compat_check` (261017-insights-query U4): reading the query
//! set ([`queries`]), the export's settings and the checks of its folder and
//! period ([`settings`]), reading one exported Logs Insights result
//! ([`export`]), writing the app's values the way Logs Insights writes them
//! ([`format`]), comparing one query ([`compare`]) and the report with the
//! exit code ([`report`]).
//!
//! Everything here is pure logic over texts and values, tested with sample
//! data made in the tests; nothing connects to AWS (U4:BR1.1, NFR4).

pub mod compare;
pub mod export;
pub mod format;
pub mod queries;
pub mod report;
pub mod settings;
