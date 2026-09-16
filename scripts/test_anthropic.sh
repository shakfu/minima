#!/usr/bin/env bash
# Live test against Anthropic, which speaks anthropic-messages natively.
#
#   ANTHROPIC_API_KEY=... scripts/test_anthropic.sh
#   MODEL=claude-sonnet-5 scripts/test_anthropic.sh
#
# This is the native Messages API, not the OpenAI-compatibility route the earlier version of this
# script used: x-api-key rather than bearer auth, content blocks rather than flat messages, and
# the SSE event name carries the meaning of each frame.
#
# Extended thinking and explicit cache_control are still not requested; see TODO.md.

PROVIDER=anthropic
KEY_ENV=ANTHROPIC_API_KEY
# Cheapest tool-capable Claude. Dated ids are the stable spelling, and MODEL overrides.
DEFAULT_MODEL=claude-haiku-4-5-20251001

. "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
run_suite
