use crate::ValueOrPanic;
use colored::ColoredString;
use colored::Colorize;

/// Trait to extend `Option` with custom unwrap methods that panic with styled messages.
///
/// ### Example
///
/// ```should_panic
/// use colored::Colorize;
/// use pidcat::ValueOrPanic;
///
/// let option: Option<i32> = None;
/// let value = option.unwrap_or_panic("Custom panic message");
///
/// let option: Option<i32> = None;
/// let value = option.unwrap_or_panic_with("Custom panic message", |msg| msg.red().bold());
/// ```
impl<T> ValueOrPanic<T> for Option<T> {
    /// Unwraps an `Option` with a custom panic message.
    ///
    /// Instead of panicking with a default message, this method panics with a custom message.
    ///
    /// ### Example
    ///
    /// ```should_panic
    /// use pidcat::ValueOrPanic;
    ///
    /// let option: Option<i32> = None;
    /// let value = option.unwrap_or_panic("Custom panic message");
    /// ```
    fn unwrap_or_panic(self, msg: &str) -> T {
        match self {
            Some(value) => value,
            None => {
                let msg_str = msg.to_string().red().bold();
                panic!("{}", msg_str)
            }
        }
    }

    /// Unwraps an `Option` with a custom panic message and style.
    ///
    /// ### Example
    ///
    /// ```should_panic
    /// use colored::Colorize;
    /// use pidcat::ValueOrPanic;
    ///
    /// let option: Option<i32> = None;
    /// let value = option.unwrap_or_panic_with("Custom panic message", |msg| msg.red().bold());
    /// ```
    fn unwrap_or_panic_with(self, msg: &str, style: fn(&str) -> ColoredString) -> T {
        match self {
            Some(value) => value,
            None => {
                let msg_str = style(msg);
                panic!("{}", msg_str)
            }
        }
    }
}
