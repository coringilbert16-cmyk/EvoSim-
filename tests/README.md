# Tests

Long-term test boundary:
- geometry/
- bob/
- chemistry/
- organism/
- construction/
- integration/

Current Rust contract/unit tests remain under src/ during migration because they exercise private crate internals. Moving them into Cargo integration-test targets would require an API-boundary change rather than a file organization change.
