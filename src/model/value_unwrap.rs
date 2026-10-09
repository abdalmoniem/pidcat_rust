// Copyright (c) AbdAlMoniem AlHifnawy <hifnawy_moniem@hotmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

//! Panic helpers that replace `unwrap` with caller-chosen messages (and optional styling).

use colored::ColoredString;

/// Unwrap [`Result`] or [`Option`] values with explicit panic messages.
///
/// Implemented for [`Option`] in [`crate::model::option_unwrap`] and [`Result`] in
/// [`crate::model::result_unwrap`].
///
/// ### Example
///
/// ```should_panic
/// use colored::Colorize;
/// use pidcatrs::ValueOrPanic;
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
    /// Unwraps with `msg` as the panic payload (styled by the type's implementation).
    ///
    /// ### Example
    ///
    /// ```should_panic
    /// use pidcatrs::ValueOrPanic;
    ///
    /// let result: Result<i32, &str> = Err("Oops");
    /// let value = result.unwrap_or_panic("Custom panic message");
    ///
    /// let option: Option<i32> = None;
    /// let value = option.unwrap_or_panic("Custom panic message");
    /// ```
    #[track_caller]
    fn unwrap_or_panic(self, msg: &str) -> T;

    /// Unwraps with `msg` and applies `style` before panicking.
    ///
    /// ### Example
    ///
    /// ```should_panic
    /// use colored::Colorize;
    /// use pidcatrs::ValueOrPanic;
    ///
    /// let result: Result<i32, &str> = Err("Oops");
    /// let value = result.unwrap_or_panic_with("Custom panic message", |msg| msg.red().bold());
    ///
    /// let option: Option<i32> = None;
    /// let value = option.unwrap_or_panic_with("Custom panic message", |msg| msg.red().bold());
    /// ```
    #[track_caller]
    fn unwrap_or_panic_with(self, msg: &str, style: fn(&str) -> ColoredString) -> T;
}
