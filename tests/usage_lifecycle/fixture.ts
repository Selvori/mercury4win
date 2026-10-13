export const fixture = {
  "from_utc": "2026-10-12T12:00:00Z",
  "to_utc": "2026-10-13T00:00:00Z",
  "totals": {
    "request_count": 3,
    "succeeded": 1,
    "failed": 1,
    "cancelled": 1,
    "timed_out": 0,
    "missing_usage": 1,
    "prompt_tokens": 8,
    "completion_tokens": 12,
    "total_tokens": 20,
    "legacy_events": 1,
    "legacy_tokens": 35
  },
  "groups": [
    {
      "id": "primary",
      "label": "renamed current",
      "archived": true,
      "metrics": {
        "request_count": 3,
        "succeeded": 1,
        "failed": 1,
        "cancelled": 1,
        "timed_out": 0,
        "missing_usage": 1,
        "prompt_tokens": 8,
        "completion_tokens": 12,
        "total_tokens": 20,
        "legacy_events": 0,
        "legacy_tokens": 0
      }
    },
    {
      "id": "snapshot:default",
      "label": "default",
      "archived": false,
      "metrics": {
        "request_count": 0,
        "succeeded": 0,
        "failed": 0,
        "cancelled": 0,
        "timed_out": 0,
        "missing_usage": 0,
        "prompt_tokens": 0,
        "completion_tokens": 0,
        "total_tokens": 0,
        "legacy_events": 1,
        "legacy_tokens": 35
      }
    }
  ],
  "daily": [
    {
      "id": "2026-10-12",
      "label": "2026-10-12",
      "archived": false,
      "metrics": {
        "request_count": 3,
        "succeeded": 1,
        "failed": 1,
        "cancelled": 1,
        "timed_out": 0,
        "missing_usage": 1,
        "prompt_tokens": 8,
        "completion_tokens": 12,
        "total_tokens": 20,
        "legacy_events": 1,
        "legacy_tokens": 35
      }
    }
  ],
  "persistence_failures": 0
};
