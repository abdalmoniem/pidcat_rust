use crate::ValueOrPanic;
use colored::ColoredString;
use colored::Colorize;

/// Trait to extend `Result` with custom unwrap methods that panic with styled messages.
///
/// ### Example
///
/// ```should_panic
/// use colored::Colorize;
/// use pidcat::ValueOrPanic;
///
/// let result: Result<i32, &str> = Err("Oops");
/// let value = result.unwrap_or_panic("Custom panic message");
///
/// let result: Result<i32, &str> = Err("Oops");
/// let value = result.unwrap_or_panic_with("Custom panic message", |msg| msg.red().bold());
/// ```
impl<T, E> ValueOrPanic<T> for Result<T, E>
where
    E: std::fmt::Debug,
{
    /// Unwraps a `Result` with a custom panic message.
    ///
    /// Instead of panicking with a default message, this method panics with a custom message.
    ///
    /// ### Example
    ///
    /// ```should_panic
    /// use pidcat::ValueOrPanic;
    ///
    /// let result: Result<i32, &str> = Err("Oops");
    /// let value = result.unwrap_or_panic("Custom panic message");
    /// ```
    fn unwrap_or_panic(self, msg: &str) -> T {
        match self {
            Ok(value) => value,
            Err(err) => {
                let msg_str = msg.to_string().red().bold();
                let err_str = format!("{:?}", err).red().bold();

                panic!("{msg_str}\n{err_str}")
            }
        }
    }

    /// Unwraps a `Result` with a custom panic message and style.
    ///
    /// ### Example
    ///
    /// ```should_panic
    /// use colored::Colorize;
    /// use pidcat::ValueOrPanic;
    ///
    /// let result: Result<i32, &str> = Err("Oops");
    /// let value = result.unwrap_or_panic_with("Custom panic message", |msg| msg.red().bold());
    /// ```
    fn unwrap_or_panic_with(self, msg: &str, style: fn(&str) -> ColoredString) -> T {
        match self {
            Ok(value) => value,
            Err(err) => {
                let msg_str = style(msg);
                let err_str = style(&format!("{:?}", err));

                panic!("{msg_str}\n{err_str}")
            }
        }
    }
}
