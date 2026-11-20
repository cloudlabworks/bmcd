// Copyright 2026 Oleg Tsarev
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//! The line speed of a node's console.
//!
//! The speed is a per-node setting, `uart_baud` in the node information. This
//! module is the one place that says what a valid value is and what a node
//! without a setting runs at.

use thiserror::Error;

/// The speed of a node's console when none was set: what a Turing RK1 and a
/// Raspberry Pi compute module use out of the box.
pub const DEFAULT_BAUD: u32 = 115_200;

/// The speeds a node's console may be set to: the standard rates of a Linux
/// serial port, up to 3 Mbaud.
/// Anything else is refused rather than rounded to something the node cannot
/// hear, which would show as garbage on the console and no error anywhere.
pub const STANDARD_BAUD_RATES: [u32; 21] = [
    300, 600, 1_200, 2_400, 4_800, 9_600, 19_200, 38_400, 57_600, 115_200, 230_400, 460_800,
    500_000, 576_000, 921_600, 1_000_000, 1_152_000, 1_500_000, 2_000_000, 2_500_000, 3_000_000,
];

/// A speed that is not one of [`STANDARD_BAUD_RATES`]. A caller's mistake,
/// reported to the client as such.
#[derive(Debug, Error, PartialEq, Eq)]
#[error(
    "{0} is not a supported serial speed; use one of 300, 600, 1200, 2400, 4800, 9600, 19200, \
     38400, 57600, 115200, 230400, 460800, 500000, 576000, 921600, 1000000, 1152000, 1500000, \
     2000000, 2500000 or 3000000 baud"
)]
pub struct InvalidBaud(pub u32);

/// Checks a speed against [`STANDARD_BAUD_RATES`].
pub fn validate_baud(baud: u32) -> Result<u32, InvalidBaud> {
    if STANDARD_BAUD_RATES.contains(&baud) {
        Ok(baud)
    } else {
        Err(InvalidBaud(baud))
    }
}

/// The speed a node runs at, given what is stored for it. A stored value that
/// is no longer valid (a settings file from elsewhere) is not used: the port
/// opens at the default and the log says so.
pub fn effective_baud(stored: Option<u32>) -> u32 {
    match stored {
        None => DEFAULT_BAUD,
        Some(baud) => validate_baud(baud).unwrap_or_else(|e| {
            tracing::warn!("{e}; using {DEFAULT_BAUD}");
            DEFAULT_BAUD
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_rates_are_accepted() {
        for baud in [9600, 115_200, 460_800, 1_500_000, 3_000_000] {
            assert_eq!(validate_baud(baud), Ok(baud));
        }
    }

    #[test]
    fn garbage_is_refused_with_a_clear_message() {
        for baud in [0, 1, 115_201, 1_499_999, 3_000_001, u32::MAX] {
            let err = validate_baud(baud).unwrap_err();
            assert_eq!(err, InvalidBaud(baud));
            let msg = err.to_string();
            assert!(msg.contains(&baud.to_string()), "{msg}");
            assert!(msg.contains("1500000"), "{msg}");
        }
    }

    #[test]
    fn the_list_of_rates_in_the_message_is_the_list_that_is_checked() {
        let msg = InvalidBaud(0).to_string();
        for rate in STANDARD_BAUD_RATES {
            assert!(
                msg.contains(&rate.to_string()),
                "{rate} missing from: {msg}"
            );
        }
    }

    #[test]
    fn an_unset_or_unusable_speed_is_the_default() {
        assert_eq!(effective_baud(None), DEFAULT_BAUD);
        assert_eq!(effective_baud(Some(1_500_000)), 1_500_000);
        assert_eq!(effective_baud(Some(12_345)), DEFAULT_BAUD);
    }
}
