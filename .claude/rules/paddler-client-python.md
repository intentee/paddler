---
paths:
  - "paddler_client_python/**"
---

# Paddler Client Context

- `paddler_client_python` provides JavaScript client that connects to `paddler_balancer`
- It must provide a way to connect to only, specifically balancer's inference address (without the need to connect to management service at the same time)
- It must provide a way to connect to only, specifically balancer's management address (without the need to connect to inference service at the same time)
- Paddler client must support all Paddler's native endpoints
- It must not implement OpenAI compatibility client
