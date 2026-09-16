"""A fake provider, for the behaviour that only shows up over the network.

Two modes, because the two things worth testing are a working gateway and a broken one:

  full       /models serves a one-entry list; the completion route streams a reply
  no-models  /models returns 404; the completion route still streams a reply

--dialect picks the wire format of the streamed response: chat, responses or messages. Each
streams the same reply and one tool call split across frames, so the harness reassembles
fragments rather than being handed a finished call.

Binds an ephemeral port and prints "PORT <n>" on stdout once listening, so the caller never has
to guess a port or poll for readiness. Request bodies go to --capture, GET paths to --gets.
"""

import argparse
import json
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

REPLY = "live path works"
ARGS_HEAD = '{"pa'
ARGS_TAIL = 'th":"notes.txt"}'


def chat_frames(reply_only):
    def call(fragment, opening):
        entry = {"index": 0, "function": {"arguments": fragment}}
        if opening:
            entry["id"] = "c1"
            entry["function"]["name"] = "read"
        return {"choices": [{"delta": {"tool_calls": [entry]}}]}

    calls = [] if reply_only else [(None, call(ARGS_HEAD, True)),
                                   (None, call(ARGS_TAIL, False))]
    return [
        (None, {"choices": [{"delta": {"content": REPLY}}]}),
        *calls,
        (None, {"choices": [{"delta": {}}],
                "usage": {"prompt_tokens": 9, "completion_tokens": 3, "total_tokens": 12}}),
        (None, "[DONE]"),
    ]


def responses_frames(reply_only):
    def delta(fragment):
        return {"type": "response.function_call_arguments.delta",
                "item_id": "item_1", "delta": fragment}

    calls = [] if reply_only else [
        (None, {"type": "response.output_item.added",
                "item": {"type": "function_call", "id": "item_1",
                         "call_id": "call_1", "name": "read"}}),
        (None, delta(ARGS_HEAD)),
        (None, delta(ARGS_TAIL)),
    ]
    return [
        (None, {"type": "response.output_text.delta", "delta": REPLY}),
        *calls,
        (None, {"type": "response.completed",
                "response": {"usage": {"input_tokens": 9, "output_tokens": 3}}}),
    ]


def messages_frames(reply_only):
    def delta(fragment):
        return ("content_block_delta",
                {"index": 1, "delta": {"type": "input_json_delta",
                                       "partial_json": fragment}})

    return [
        ("message_start", {"message": {"usage": {"input_tokens": 9, "output_tokens": 0}}}),
        ("content_block_start", {"index": 0, "content_block": {"type": "text", "text": ""}}),
        ("content_block_delta",
         {"index": 0, "delta": {"type": "text_delta", "text": REPLY}}),
        ("content_block_stop", {"index": 0}),
        *([] if reply_only else [
            ("content_block_start",
             {"index": 1,
              "content_block": {"type": "tool_use", "id": "toolu_1", "name": "read"}}),
            delta(ARGS_HEAD),
            delta(ARGS_TAIL),
            ("content_block_stop", {"index": 1}),
        ]),
        ("message_delta", {"usage": {"output_tokens": 3}}),
        ("message_stop", {}),
    ]


def frames(dialect, reply_only):
    return {
        "chat": chat_frames,
        "responses": responses_frames,
        "messages": messages_frames,
    }[dialect](reply_only)


def handler(args):
    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def log_message(self, *_):
            pass

        def do_GET(self):
            if args.gets:
                with open(args.gets, "a") as f:
                    f.write(self.path + "\n")
            if args.mode == "no-models":
                self.send_response(404)
                self.send_header("content-length", "0")
                self.end_headers()
                return
            body = json.dumps(
                {"data": [{"id": "fake-model", "context_length": 32000}]}
            ).encode()
            self.send_response(200)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_POST(self):
            raw = self.rfile.read(int(self.headers.get("content-length", 0)))
            if args.capture:
                with open(args.capture, "wb") as f:
                    f.write(raw)
            payload = b""
            for name, data in frames(args.dialect, args.reply_only):
                if name:
                    payload += ("event: %s\n" % name).encode()
                body = data if isinstance(data, str) else json.dumps(data)
                payload += ("data: %s\n\n" % body).encode()
            self.send_response(200)
            self.send_header("content-type", "text/event-stream")
            self.send_header("content-length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

    return Handler


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=["full", "no-models"], default="full")
    parser.add_argument(
        "--dialect", choices=["chat", "responses", "messages"], default="chat"
    )
    parser.add_argument(
        "--reply-only", action="store_true",
        help="stream only text, for tests that are not about tool calls",
    )
    parser.add_argument("--capture")
    parser.add_argument("--gets")
    args = parser.parse_args()

    server = HTTPServer(("127.0.0.1", 0), handler(args))
    print("PORT %d" % server.server_address[1], flush=True)
    server.serve_forever()


if __name__ == "__main__":
    sys.exit(main())
