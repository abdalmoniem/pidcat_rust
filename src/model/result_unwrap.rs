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

//! [`ValueOrPanic`] for [`Result`].

use crate::ValueOrPanic;
use colored::ColoredString;
use colored::Colorize;

/// [`ValueOrPanic`] implementation for [`Result`].
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
/// let result: Result<i32, &str> = Err("Oops");
/// let value = result.unwrap_or_panic_with("Custom panic message", |msg| msg.red().bold());
/// ```
impl<T, E> ValueOrPanic<T> for Result<T, E>
where
    E: std::fmt::Debug,
{
    /// Returns `Ok` or panics with bold red `msg` and a debug-formatted error line.
    ///
    /// ### Example
    ///
    /// ```should_panic
    /// use pidcatrs::ValueOrPanic;
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

    /// Returns `Ok` or panics with styled `msg` and styled debug error.
    ///
    /// ### Example
    ///
    /// ```should_panic
    /// use colored::Colorize;
    /// use pidcatrs::ValueOrPanic;
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
