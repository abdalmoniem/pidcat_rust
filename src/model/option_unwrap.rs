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

//! [`ValueOrPanic`] for [`Option`].

use crate::ValueOrPanic;
use colored::ColoredString;
use colored::Colorize;

/// [`ValueOrPanic`] implementation for [`Option`].
///
/// ### Example
///
/// ```should_panic
/// use colored::Colorize;
/// use pidcatrs::ValueOrPanic;
///
/// let option: Option<i32> = None;
/// let value = option.unwrap_or_panic("Custom panic message");
///
/// let option: Option<i32> = None;
/// let value = option.unwrap_or_panic_with("Custom panic message", |msg| msg.red().bold());
/// ```
impl<T> ValueOrPanic<T> for Option<T> {
    /// Returns the inner value or panics with a bold red `msg`.
    ///
    /// ### Example
    ///
    /// ```should_panic
    /// use pidcatrs::ValueOrPanic;
    ///
    /// let option: Option<i32> = None;
    /// let value = option.unwrap_or_panic("Custom panic message");
    /// ```
    fn unwrap_or_panic(self, msg: &str) -> T {
        match self {
            Some(value) => value,
            None => {
                let msg_str = msg.to_string().red().bold();
                panic!("{msg_str}")
            }
        }
    }

    /// Returns the inner value or panics with `style(msg)`.
    ///
    /// ### Example
    ///
    /// ```should_panic
    /// use colored::Colorize;
    /// use pidcatrs::ValueOrPanic;
    ///
    /// let option: Option<i32> = None;
    /// let value = option.unwrap_or_panic_with("Custom panic message", |msg| msg.red().bold());
    /// ```
    fn unwrap_or_panic_with(self, msg: &str, style: fn(&str) -> ColoredString) -> T {
        match self {
            Some(value) => value,
            None => {
                let msg_str = style(msg);
                panic!("{msg_str}")
            }
        }
    }
}
