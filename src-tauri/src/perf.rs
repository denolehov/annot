/// Performance measurement utilities.
///
/// Enable with `ANNOT_PERF=1` environment variable.

/// Times an expression and prints the duration to stderr if `ANNOT_PERF` is set.
///
/// # Example
/// ```ignore
/// let result = timed!("expensive_operation", do_something());
/// ```
macro_rules! timed {
    ($label:expr, $expr:expr) => {{
        let _start = std::time::Instant::now();
        let _result = $expr;
        if std::env::var("ANNOT_PERF").is_ok() {
            eprintln!("[perf] {}: {:?}", $label, _start.elapsed());
        }
        _result
    }};
}

pub(crate) use timed;
