use colored::ColoredString;

/// Trait to extend `Result` or `Option` with custom unwrap methods that panic with styled messages.
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
/// let option: Option<i32> = None;
/// let value = option.unwrap_or_panic("Custom panic message");
///
/// let result: Result<i32, &str> = Err("Oops");
/// let value = result.unwrap_or_panic_with("Custom panic message", |msg| msg.red().bold());
///
/// let option: Option<i32> = None;
/// let value = option.unwrap_or_panic_with("Custom panic message", |msg| msg.red().bold());
/// ```
pub trait ValueOrPanic<T> {
    /// Unwraps a `Result` or an `Option` with a custom panic message.
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
    ///
    /// let option: Option<i32> = None;
    /// let value = option.unwrap_or_panic("Custom panic message");
    /// ```
    ///
    fn unwrap_or_panic(self, msg: &str) -> T;

    /// Unwraps a `Result` or an `Option` with a custom panic message and style.
    ///
    /// ### Example
    ///
    /// ```should_panic
    /// use colored::Colorize;
    /// use pidcat::ValueOrPanic;
    ///
    /// let result: Result<i32, &str> = Err("Oops");
    /// let value = result.unwrap_or_panic_with("Custom panic message", |msg| msg.red().bold());
    ///
    /// let option: Option<i32> = None;
    /// let value = option.unwrap_or_panic_with("Custom panic message", |msg| msg.red().bold());
    /// ```
    ///
    fn unwrap_or_panic_with(self, msg: &str, style: fn(&str) -> ColoredString) -> T;
}
