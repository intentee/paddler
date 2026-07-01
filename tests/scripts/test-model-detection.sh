#!/usr/bin/env bash
#
# Test script for model detection changes in proxy_service.rs
#
# Usage:
#   ./test-model-detection.sh                          # uses defaults (localhost)
#   ./test-model-detection.sh deeplearning4.stockpulse.de
#   PADDLER_PROXY_HOST=deeplearning4.stockpulse.de ./test-model-detection.sh
#   PADDLER_PROXY_PORT=5000 PADDLER_MGMT_PORT=8085 PADDLER_MODEL=gemma4 ./test-model-detection.sh
#
# Environment variables (all optional, CLI arg overrides host):
#   PADDLER_PROXY_HOST   - reverse proxy hostname/IP   (default: localhost)
#   PADDLER_PROXY_PORT   - reverse proxy port          (default: 5000)
#   PADDLER_MGMT_HOST    - management API hostname/IP  (default: same as proxy host)
#   PADDLER_MGMT_PORT    - management API port         (default: 8085)
#   PADDLER_MODEL        - model name to test with     (default: gemma4)

# ── Resolve configuration ──

# Parse a host or full URL into scheme + host, defaulting to http://
parse_url() {
    local input="$1"
    # Strip trailing slashes
    input="${input%%/}"
    input="${input%%\?}"
    if [[ "$input" =~ ^https:// ]]; then
        echo "https ${input#https://}"
    elif [[ "$input" =~ ^http:// ]]; then
        echo "http ${input#http://}"
    else
        echo "http $input"
    fi
}

read -r PROXY_SCHEME PROXY_HOST <<< "$(parse_url "${PADDLER_PROXY_HOST:-${1:-localhost}}")"
PROXY_PORT="${PADDLER_PROXY_PORT:-5000}"

# Management defaults to same host and scheme as proxy if not explicitly set
if [ -n "${PADDLER_MGMT_HOST:-}" ]; then
    read -r MGMT_SCHEME MGMT_HOST <<< "$(parse_url "$PADDLER_MGMT_HOST")"
else
    MGMT_SCHEME="$PROXY_SCHEME"
    MGMT_HOST="$PROXY_HOST"
fi
MGMT_PORT="${PADDLER_MGMT_PORT:-8085}"
MODEL="${PADDLER_MODEL:-gemma4}"

PROXY="${PROXY_SCHEME}://${PROXY_HOST}:${PROXY_PORT}"
MANAGEMENT="${MGMT_SCHEME}://${MGMT_HOST}:${MGMT_PORT}"

PASS=0
FAIL=0

green()  { echo -e "\033[32m  ✓ $1\033[0m"; }
red()    { echo -e "\033[31m  ✗ $1\033[0m"; }
yellow() { echo -e "\033[33m  ⚠ $1\033[0m"; }
header() { echo -e "\n\033[1m── $1 ──\033[0m"; }

pass() { green "$1"; PASS=$((PASS + 1)); }
fail() { red "$1"; FAIL=$((FAIL + 1)); }

check_status() {
    local desc="$1" expected="$2" actual="$3"
    if [ "$actual" = "$expected" ]; then
        pass "$desc (HTTP $actual)"
    else
        fail "$desc (expected HTTP $expected, got HTTP $actual)"
    fi
}

# ── Banner ──

echo "======================================================"
echo "  Paddler Model Detection Test Suite"
echo "======================================================"
echo "  Proxy:      $PROXY"
echo "  Management: $MANAGEMENT"
echo "  Model:      $MODEL"
echo "======================================================"

# ── Pre-flight: check that the management API is reachable ──

header "Pre-flight checks"

MGMT_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 5 "$MANAGEMENT/api/v1/agents" 2>/dev/null)
MGMT_STATUS=${MGMT_STATUS:-000}
if [ "$MGMT_STATUS" = "200" ]; then
    pass "Management API reachable at $MANAGEMENT"
else
    red "Management API unreachable (HTTP $MGMT_STATUS) — is the balancer running?"
    echo ""
    echo "  Configure the target host with one of:"
    echo "    ./test-model-detection.sh <host>"
    echo "    PADDLER_PROXY_HOST=<host> ./test-model-detection.sh"
    echo ""
    echo "  Available environment variables:"
    echo "    PADDLER_PROXY_HOST   - reverse proxy host (default: localhost)"
    echo "    PADDLER_PROXY_PORT   - reverse proxy port (default: 5000)"
    echo "    PADDLER_MGMT_HOST    - management API host (default: same as proxy)"
    echo "    PADDLER_MGMT_PORT    - management API port (default: 8085)"
    echo "    PADDLER_MODEL        - model name to test (default: gemma4)"
    echo ""
    echo "Aborting tests."
    exit 1
fi

# Show registered agents
echo ""
echo "  Registered agents:"
curl -s "$MANAGEMENT/api/v1/agents" 2>/dev/null | python3 -m json.tool 2>/dev/null || true
echo ""

# ── Test 1: Valid request with correct model ──

header "Test 1: Valid request with correct model ($MODEL)"

BODY="{\"model\": \"$MODEL\", \"messages\": [{\"role\": \"user\", \"content\": \"Say hello in one word.\"}], \"max_tokens\": 10}"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d "$BODY" 2>/dev/null)
check_status "POST /v1/chat/completions with model=$MODEL" "200" "$HTTP_CODE"

# ── Test 2: Full functional test — actual joke response ──

header "Test 2: Functional test — tell a short joke"

RESPONSE=$(curl -s -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d "{\"model\": \"$MODEL\", \"messages\": [{\"role\": \"user\", \"content\": \"Tell a very short joke.\"}], \"max_tokens\": 50}" 2>/dev/null)

if echo "$RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); assert 'choices' in d" 2>/dev/null; then
    pass "Got valid chat completion response"
    # Print the actual reply
    CONTENT=$(echo "$RESPONSE" | python3 -c "import sys,json; print(json.load(sys.stdin)['choices'][0]['message']['content'].strip())" 2>/dev/null || true)
    echo "  Response: $CONTENT"
else
    fail "Invalid or empty response"
    echo "  Raw: ${RESPONSE:0:200}"
fi

# ── Test 3: Wrong model — should get 404 ──

header "Test 3: Wrong model (nonexistent) — expect 404"

HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d '{"model": "llama-fake-model-xyz", "messages": [{"role": "user", "content": "hi"}], "max_tokens": 5}' 2>/dev/null)
check_status "POST with wrong model" "404" "$HTTP_CODE"

# ── Test 4: Missing model field — should get 400 ──

header "Test 4: Missing model field — expect 400"

HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d '{"messages": [{"role": "user", "content": "hi"}], "max_tokens": 5}' 2>/dev/null)
check_status "POST without model field" "400" "$HTTP_CODE"

# ── Test 5: Empty body — should get 400 ──

header "Test 5: Empty request body — expect 400"

HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d '{}' 2>/dev/null)
check_status "POST with empty JSON body" "400" "$HTTP_CODE"

# ── Test 6: Invalid JSON body — should get 400 ──

header "Test 6: Invalid JSON body — expect 400"

HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d 'not json at all' 2>/dev/null)
check_status "POST with invalid JSON" "400" "$HTTP_CODE"

# ── Test 7: Non-JSON content type — model check skipped, no model found → 400 ──

header "Test 7: Non-JSON Content-Type (model check skipped, no model found)"

HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: text/plain" \
    -d "{\"model\": \"$MODEL\"}" 2>/dev/null)
# With --check-model, non-JSON Content-Type skips model extraction.
# Since no model is found, the balancer rejects with 400. This is expected.
check_status "POST with text/plain Content-Type (model not extracted)" "400" "$HTTP_CODE"

# ── Test 8: GET request — no body, no model found → 400 ──

header "Test 8: GET request (no body, no model found)"

HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X GET "$PROXY/v1/chat/completions" 2>/dev/null)
# With --check-model, GET has no body so no model is found → 400. This is expected.
check_status "GET /v1/chat/completions (no body, no model)" "400" "$HTTP_CODE"

# ── Test 9: Model in different JSON field order ──

header "Test 9: Model field not first in JSON"

BODY="{\"messages\": [{\"role\": \"user\", \"content\": \"Say hi.\"}], \"max_tokens\": 10, \"model\": \"$MODEL\"}"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d "$BODY" 2>/dev/null)
check_status "POST with model field last in JSON" "200" "$HTTP_CODE"

# ── Test 10: Model with extra whitespace in JSON ──

header "Test 10: Model with extra whitespace in JSON"

BODY="{  \"model\"  :  \"$MODEL\"  ,  \"messages\"  :  [  {  \"role\"  :  \"user\"  ,  \"content\"  :  \"hi\"  }  ]  ,  \"max_tokens\"  :  5  }"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d "$BODY" 2>/dev/null)
check_status "POST with extra whitespace in JSON" "200" "$HTTP_CODE"

# ── Test 11: Completions endpoint (not chat) ──

header "Test 11: /v1/completions endpoint"

HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/v1/completions" \
    -H "Content-Type: application/json" \
    -d "{\"model\": \"$MODEL\", \"prompt\": \"Say hi\", \"max_tokens\": 5}" 2>/dev/null)
check_status "POST /v1/completions with model=$MODEL" "200" "$HTTP_CODE"

# ── Test 12: Legacy llama.cpp endpoint /completion ──

header "Test 12: Legacy /completion endpoint"

HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/completion" \
    -H "Content-Type: application/json" \
    -d "{\"model\": \"$MODEL\", \"prompt\": \"Say hi\", \"n_predict\": 5}" 2>/dev/null)
check_status "POST /completion with model=$MODEL" "200" "$HTTP_CODE"

# ── Test 13: Payload >64KB — needle-in-haystack ──

header "Test 13: Payload >64KB (needle-in-haystack, verifies full body forwarding)"

# Generate a ~70KB payload: model field first, padding in the middle, needle at the end.
# If the full body reaches the upstream, the model should be able to find the needle.
NEEDLE="PADDLER_TEST_SECRET_42"
PADDING=$(python3 -c "print('x' * 70000)")
BIG_BODY=$(printf '{"model": "%s", "messages": [{"role": "user", "content": "%s The secret code is %s. Reply with only the secret code."}], "max_tokens": 20}' "$MODEL" "$PADDING" "$NEEDLE")

BIG_RESPONSE=$(curl -s -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d "$BIG_BODY" 2>/dev/null)

BIG_HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d "$BIG_BODY" 2>/dev/null)

if [ "$BIG_HTTP_CODE" = "200" ]; then
    # Check if the model found the needle in the response
    if echo "$BIG_RESPONSE" | grep -q "$NEEDLE"; then
        pass "Got HTTP 200 and model found the needle (full body forwarded)"
    else
        yellow "Got HTTP 200 but model did NOT find the needle"
        yellow "The body may have been truncated or the model ignored it"
        echo "  Response: $(echo "$BIG_RESPONSE" | python3 -c "import sys,json; print(json.load(sys.stdin).get('choices',[{}])[0].get('message',{}).get('content','').strip())" 2>/dev/null || echo "(parse error)")"
        fail "Needle not found in response"
    fi
else
    fail "Got HTTP $BIG_HTTP_CODE (expected 200)"
fi

# ── Test 14: Image input (base64 in content, non-UTF-8 safe) ──

header "Test 14: Vision request with base64 image in content"

# Base64 image data (small 1x1 red PNG) — valid UTF-8 but large binary-ish payload
BASE64_IMG="iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
VISION_BODY=$(printf '{"model": "%s", "messages": [{"role": "user", "content": [{"type": "text", "text": "Describe this image."}, {"type": "image_url", "image_url": {"url": "data:image/png;base64,%s"}}]}], "max_tokens": 10}' "$MODEL" "$BASE64_IMG")
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d "$VISION_BODY" 2>/dev/null)
# The model field should be extracted correctly even with base64 in the body.
# Upstream may reject if it's not a vision model, but balancer should NOT return 400.
check_status "POST with base64 image in content" "200" "$HTTP_CODE"

# ── Test 15: Model field appears inside content string (false-positive check) ──

header "Test 15: model keyword inside content string (no false positive)"

# The word "model" appears in the content, but the real model field is $MODEL
FALSE_POSITIVE_BODY=$(printf '{"model": "%s", "messages": [{"role": "user", "content": "I want to use a different model called llama3."}], "max_tokens": 10}' "$MODEL")
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d "$FALSE_POSITIVE_BODY" 2>/dev/null)
check_status "POST with model keyword in content (should extract $MODEL)" "200" "$HTTP_CODE"

# ── Test 16: Streaming request ──

header "Test 16: Streaming request (stream: true)"

STREAM_BODY=$(printf '{"model": "%s", "messages": [{"role": "user", "content": "Say hi."}], "max_tokens": 10, "stream": true}' "$MODEL")
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d "$STREAM_BODY" 2>/dev/null)
check_status "POST with stream: true" "200" "$HTTP_CODE"

# ── Test 17: GET /v1/models (listing, no body, no model check) ──

header "Test 17: GET /v1/models (model listing endpoint)"

HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X GET "$PROXY/v1/models" 2>/dev/null)
# This is not a slots endpoint, so model check should not apply.
# Upstream should return 200 with the model list.
check_status "GET /v1/models" "200" "$HTTP_CODE"

# ── Test 18: Model with special characters in name ──

header "Test 18: Model name with special characters (hyphens, dots)"

# Test with a model name that has hyphens/dots — should still be extracted
SPECIAL_MODEL_BODY='{"model": "gemma-2-9b-it", "messages": [{"role": "user", "content": "hi"}], "max_tokens": 5}'
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$PROXY/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d "$SPECIAL_MODEL_BODY" 2>/dev/null)
# This model doesn't exist on the upstream, so we expect 404 (model not found), not 400 (extraction failed)
check_status "POST with model name containing hyphens" "404" "$HTTP_CODE"

# ── Summary ──

header "Summary"
echo "  Passed: $PASS"
echo "  Failed: $FAIL"
echo "  Total:  $((PASS + FAIL))"
echo ""

if [ "$FAIL" -gt 0 ]; then
    echo "  Some tests failed. Review the output above."
    exit 1
else
    echo "  All tests passed!"
    exit 0
fi
