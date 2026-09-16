#!/usr/bin/env bash
# Live test against OpenRouter, which speaks openai-chat.
#
#   OPENROUTER_API_KEY=... scripts/test_openrouter.sh
#   MODEL=qwen/qwen3.7-flash scripts/test_openrouter.sh
#
# OpenRouter fronts many upstreams behind one endpoint, so this is the broadest single check of
# what minima puts on the wire.

PROVIDER=openrouter
KEY_ENV=OPENROUTER_API_KEY
# ':batch' variants do not stream; ':free' ones rate-limit, which would make a failure ambiguous
# between minima and the provider.
DEFAULT_MODEL=openai/gpt-5-nano

. "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
run_suite
