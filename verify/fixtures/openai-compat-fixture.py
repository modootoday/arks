#!/usr/bin/env python3
"""Deterministic OpenAI-compatible fixture for the B4 Docker gate."""

import argparse
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--listen", default="127.0.0.1")
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--log", type=Path, required=True)
    args = parser.parse_args()
    args.log.parent.mkdir(parents=True, exist_ok=True)

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, _format: str, *_values: object) -> None:
            return

        def do_GET(self) -> None:
            if self.path.rstrip("/") != "/v1/models":
                self.send_error(404)
                return
            self.reply({"object": "list", "data": [{"id": "arks-fixture"}]})

        def do_POST(self) -> None:
            if self.path.rstrip("/") != "/v1/chat/completions":
                self.send_error(404)
                return
            size = int(self.headers.get("content-length", "0"))
            request = json.loads(self.rfile.read(size))
            messages = request.get("messages", [])
            with args.log.open("a", encoding="utf-8") as output:
                output.write(json.dumps(request, sort_keys=True) + "\n")
            users = [m.get("content", "") for m in messages if m.get("role") == "user"]
            assistants = [
                m.get("content", "") for m in messages if m.get("role") == "assistant"
            ]
            if "B4_REVIEW_TOKEN" in " ".join(users):
                content = "review-result: no invented Claimed or PASS; fixture token observed"
            elif len(users) >= 2:
                if assistants != ["fixture-turn-1"]:
                    self.reply(
                        {"error": {"message": "second turn omitted assistant history"}},
                        status=422,
                    )
                    return
                content = "fixture-turn-2-history-ok"
            else:
                content = "fixture-turn-1"
            self.reply(
                {
                    "id": "chatcmpl-b4-fixture",
                    "object": "chat.completion",
                    "choices": [
                        {
                            "index": 0,
                            "message": {"role": "assistant", "content": content},
                            "finish_reason": "stop",
                        }
                    ],
                }
            )

        def reply(self, value: object, status: int = 200) -> None:
            body = json.dumps(value, separators=(",", ":")).encode()
            self.send_response(status)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    ThreadingHTTPServer((args.listen, args.port), Handler).serve_forever()


if __name__ == "__main__":
    main()
