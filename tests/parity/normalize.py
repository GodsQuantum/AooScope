from copy import deepcopy


def normalize(path, payload):
    """Normalize only explicitly version-specific fields."""
    value = deepcopy(payload)
    if path in {"/api/health", "/api/status"} and isinstance(value, dict):
        value["version"] = "<version>"
    return value
