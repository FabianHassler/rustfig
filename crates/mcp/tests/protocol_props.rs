#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Arbitrary protocol input must never panic the server.

use proptest::prelude::*;
use rustfig_mcp::Server;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]
    #[test]
    fn random_lines(line in "[ -~]{0,400}") {
        let mut s = Server::new();
        let _ = s.handle_line(&line);
    }

    #[test]
    fn random_tool_calls(name in "rustfig_(execute|commands|render|document|x)", cmd in "[a-z_.]{0,20}", n in -1e12f64..1e12) {
        let mut s = Server::new();
        let msg = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": name, "arguments": {"command": cmd, "params": {"x": n, "y": n, "width": n, "height": n, "dpi": n}}}});
        let r = s.handle(&msg).unwrap();
        prop_assert!(r.get("result").is_some());
    }
}
