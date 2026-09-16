#!/usr/bin/env bash
# Live test against OpenAI, which speaks openai-responses.
#
#   OPENAI_API_KEY=... scripts/test_openai.sh
#   MODEL=gpt-5-nano scripts/test_openai.sh
#
# Responses is the dialect OpenAI itself prefers, and the one hax routes this provider through.
# minima sends store:false, so nothing is retained server-side.

PROVIDER=openai
KEY_ENV=OPENAI_API_KEY
DEFAULT_MODEL=gpt-5-nano

. "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
run_suite
