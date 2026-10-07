---
paths:
  - "paddler_typesafe_translation/**"
---

# Paddler TypeSafe Translation Context

- `paddler_typesafe_translation` translates TypeSafe System One requests (`POST /v1/systemone`) into Paddler decisions, and decision answers back into TypeSafe answers
- it reproduces kev's `kev.api` exactly: JSON content renders with Python `str()` semantics (Python float repr, `True`/`False`, `lstrip`), noul/choice/score questions become option lists, and answers carry kev's confidences rounded to 4 decimals
- `fixtures/typesafe_reference.json`, written by `paddler_kev_converter_python` from `kev.api`, is the golden contract its tests check
- question and criteria order follow the request, which `serde_json`'s `preserve_order` keeps
- it never depends on actix or the balancer; the TypeSafe HTTP service builds on it
- `TypeSafeTranslationError` is its single error enum
