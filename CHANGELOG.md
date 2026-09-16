# Changelog

## 0.2.0 - 2026-09-16

### Changed

- A cancelled `-p` run exits 130 instead of 0, so a script can tell it from a finished run.

- `--api-key` and `MINIMA_API_KEY` now require `--provider`. A key does not name its vendor, so autoselect could send an OpenAI key to `openrouter.ai`. Requiring the flag was chosen over ignoring the key under autoselect, which would silently drop a key the user set.

- `bash` runs `bash -c` instead of `bash -lc`. Login files added startup time and printed into tool results; commands now inherit minima's environment, including `PATH`.

- The `bash` tool description tells the model not to wrap commands in another shell. OpenAI models wrapped them in `bash -lc` in live runs.

- `bash` runs each command in its own process group. A timeout or cancel kills the group, not only `bash`, and output produced before a timeout is kept.

- Background jobs outlive their call but not minima. The call result says they are still running, and minima kills them when it exits, including on SIGHUP (exit 129) and SIGTERM (exit 143). This was chosen over killing them when the call ends, which would stop a server started in one call from serving the next.

- The system prompt names `bash` as the command shell instead of `$SHELL`, which on macOS is zsh.

- The model is remembered per provider only after its first turn streams. A mistyped `--model` is no longer reused by later runs.

- Requests time out: 10 s to connect, 300 s between reads. A stalled stream no longer hangs `-p`. The read timeout bounds silence, not stream length.

- A full context window is refused before sending. The turn that fills it is kept, and its tool calls are answered but not run.

- Ctrl-C cancels a REPL turn, as Esc does. Cancelling while a request is pending, or during retry backoff, takes effect at once.

- A provider asking to retry after more than 60 s is reported instead of waited out.

- History excludes only `/quit`. Prompts starting with a path, such as `/etc/hosts`, are recorded.

- `read` streams the file, so memory follows the lines returned, not the file size. A multi-line `edit` matches CRLF files.

- The README states that there is no approval gate, and no longer cites a line target.

### Fixed

- A cancel between tool calls left later calls without results. Every dialect rejects such a transcript, so each following request in the REPL session failed until restart. Every call now gets a result, including calls skipped for a cancel, a full window, or the output limit.

- A cancelled tool call still sent the next request, which was paid for and then dropped.

- A cancel could lose to already-buffered stream events, keeping part of the cancelled turn.

- A `bash` command that started a background job waited for the full timeout. The job held the stdout pipe open, so EOF never arrived. Reading now stops 100 ms after `bash` exits.

- `edit` with an empty `old` rewrote the file: an empty pattern matches between every character. It is now refused.

- A response cut off at the output token limit looked like malformed tool arguments, and the model retried until `--max-turns`. All three dialects now report the stop. Cut-off calls are not run, their arguments are stored as `{}`, and cut-off text is an error.

- Assistant text and error bodies reached the terminal with escape sequences intact. Text from a file the model read could retitle the terminal or write to the clipboard.

- Chat Completions: calls without an `index` merged into one; mid-stream `error` chunks were dropped; an empty assistant turn was sent without `content`.

- Tool output with stdout lacking a trailing newline ran into the first stderr line.

## 0.1.0 - 2026-09-16

Initial release.
