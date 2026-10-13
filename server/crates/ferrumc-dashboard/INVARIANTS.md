# Invariants: ferrumc-dashboard

- The application selects the socket address; wildcard and specific-interface
  bindings are supported, with binding failures returned as typed I/O errors.
- All endpoints remain read-only. GET/HEAD are allowed; other methods are rejected
  before a data handler or static-file service runs.
- Snapshot publication and HTTP reads cannot mutate simulation state.
