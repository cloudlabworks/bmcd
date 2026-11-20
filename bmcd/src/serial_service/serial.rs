// Copyright 2023 Turing Machines
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
//! Handlers for UART connections to/from nodes
use std::{ops::Index, path::PathBuf};

use std::sync::Arc;

use super::baud::DEFAULT_BAUD;
use super::console_pty::{console_link_name, ConsolePty, CONSOLE_LINK_DIR};
use super::serial_handler::Handler;
use crate::hal::NodeId;
use crate::serial_service::serial_handler::HandlerState;
use tokio_serial::{DataBits, Parity, StopBits};
use tracing::{error, info, warn};

/// Collection of [`crate::serial_service::serial_handler::Handler`]
#[derive(Debug)]
pub struct SerialConnections {
    handlers: Vec<Handler>,
}

impl SerialConnections {
    /// Opens the four console ports, each at its speed from `bauds` (index 0
    /// is node 1), and gives each node its shared console device
    /// `/dev/ttyNodeN`. Must be called from within the tokio runtime.
    pub fn new(bauds: [u32; 4]) -> Self {
        Self::with_console_dir(bauds, std::path::Path::new(CONSOLE_LINK_DIR))
    }

    fn with_console_dir(bauds: [u32; 4], link_dir: &std::path::Path) -> Self {
        let paths = get_serial_devices();

        let collection = paths.iter().enumerate().map(|(i, path)| {
            let node = i + 1;
            let mut handler = Handler::new(
                node,
                path,
                bauds.get(i).copied().unwrap_or(DEFAULT_BAUD),
                DataBits::Eight,
                Parity::None,
                StopBits::One,
            );

            // The console device is made before the worker starts, which is
            // what attaches it. If the port then does not open it is taken
            // off again, and its link goes with it: a link to a console that
            // is not there would send a tool to a terminal nothing ever
            // writes to.
            match ConsolePty::create(&link_dir.join(console_link_name(node))) {
                Ok(console) => handler.set_console(Some(Arc::new(console))),
                Err(e) => warn!("[node {}] no shared console device: {}", node, e),
            }

            match handler.run() {
                Ok(()) => {
                    if let Some(console) = handler.console() {
                        info!(
                            "[node {}] shared console device {} (slave {}), use it instead of {}",
                            node,
                            console.link().display(),
                            console.slave_path().display(),
                            path
                        );
                    }
                }
                Err(e) => {
                    error!("handler run error: {}", e);
                    handler.set_console(None);
                }
            }

            handler
        });

        SerialConnections {
            handlers: collection.collect(),
        }
    }

    pub fn get_state(&self) -> Vec<HandlerState> {
        self.handlers.iter().map(Handler::get_state).collect()
    }

    /// Applies a new speed to the console port of `node`, without dropping
    /// the clients attached to it.
    pub fn set_baud_rate(&self, node: NodeId, baud: u32) {
        self.handlers[node as usize].set_baud_rate(baud);
    }
}

impl Index<NodeId> for SerialConnections {
    type Output = Handler;

    fn index(&self, index: NodeId) -> &Self::Output {
        assert!(
            self.handlers.len() == 4,
            "Serial connections not initialized"
        );
        &self.handlers[index as usize]
    }
}

/// This is a quick and dirty way to detect which serial devices to load. At some point in time the
/// mapping of ttySx devices to the uart ports changed to align them numerically with the nodes
/// switched the numbering of
fn get_serial_devices() -> [&'static str; 4] {
    let device_type = PathBuf::from("/sys/class/tty/ttyS3/device/of_node/device_type");
    if matches!(std::fs::read_to_string(device_type), Ok(str) if str == "uart3") {
        ["/dev/ttyS2", "/dev/ttyS1", "/dev/ttyS4", "/dev/ttyS5"]
    } else {
        ["/dev/ttyS1", "/dev/ttyS2", "/dev/ttyS3", "/dev/ttyS4"]
    }
}
