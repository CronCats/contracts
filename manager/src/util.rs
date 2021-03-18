// convenient logger
#[macro_export]
macro_rules! logger {
  ($($arg:tt)*) => ({
    let log_message = format!($($arg)*);
    let log = log_message.as_bytes();
    env::log(&log)
  })
}
