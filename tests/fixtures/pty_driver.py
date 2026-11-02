"""Run a command on a pseudo-terminal, as the REPL needs one, and print what it drew.

Waits for --until in the output, compared without whitespace, then --settle seconds more, then
sends Ctrl-D. Answers the cursor-position query the inline viewport sends at start, which a real
terminal would answer.
Exits 3 if --until never appears within --timeout; otherwise with the command's status.
"""

import argparse
import fcntl
import os
import pty
import re
import select
import struct
import sys
import termios
import time

DSR = b"\x1b[6n"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--until", required=True)
    parser.add_argument("--settle", type=float, default=0.0)
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command

    pid, fd = pty.fork()
    if pid == 0:
        os.execvp(command[0], command)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))

    seen = b""
    deadline = time.monotonic() + args.timeout
    quit_at = None
    sent = False
    while True:
        now = time.monotonic()
        if now > deadline:
            os.kill(pid, 9)
            sys.stdout.write(plain(seen))
            sys.exit(3)
        if quit_at is None and squeeze(args.until.encode()) in squeeze(strip(seen)):
            quit_at = now + args.settle
        if quit_at is not None and not sent and now >= quit_at:
            os.write(fd, b"\x04")
            sent = True
        ready, _, _ = select.select([fd], [], [], 0.05)
        if not ready:
            continue
        try:
            chunk = os.read(fd, 4096)
        except OSError:
            break
        if not chunk:
            break
        # Each query is answered once; a reply of row 1, column 1 is enough for the viewport.
        for _ in range(chunk.count(DSR)):
            os.write(fd, b"\x1b[1;1R")
        seen += chunk
    _, status = os.waitpid(pid, 0)
    sys.stdout.write(plain(seen))
    if quit_at is None:
        sys.exit(3)
    sys.exit(os.waitstatus_to_exitcode(status))


def strip(data):
    return re.sub(rb"\x1b\[[0-9;?]*[ -/]*[@-~]", b"", data)


def squeeze(data):
    """The renderer moves the cursor over blank cells, so spaces between words may be absent."""
    return re.sub(rb"\s", b"", data)


def plain(data):
    return strip(data).decode("utf-8", "replace")


if __name__ == "__main__":
    main()
