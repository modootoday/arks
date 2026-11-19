#!/usr/bin/env python3
"""Minimal HTTP CONNECT proxy for gate G42 (oracle tooling, not product).

Simulates a hosted agent sandbox egress proxy: clients must reach the TLS
server through CONNECT. Hostnames in G42_HOSTS (name=ip:port,...) resolve only
here, so a client that bypasses the proxy cannot connect at all. Each tunnel
is logged to the file given as argv[2].
"""
import os
import socket
import sys
import threading

port = int(sys.argv[1])
log_path = sys.argv[2]
hosts = {}
for item in filter(None, os.environ.get("G42_HOSTS", "").split(",")):
    name, target = item.split("=", 1)
    h, p = target.rsplit(":", 1)
    hosts[name] = (h, int(p))
lock = threading.Lock()


def pipe(src, dst):
    try:
        while True:
            data = src.recv(65536)
            if not data:
                break
            dst.sendall(data)
    except OSError:
        pass
    finally:
        try:
            dst.shutdown(socket.SHUT_WR)
        except OSError:
            pass


def handle(client):
    try:
        buf = b""
        while b"\r\n\r\n" not in buf:
            chunk = client.recv(4096)
            if not chunk:
                return
            buf += chunk
        line = buf.split(b"\r\n", 1)[0].decode("latin-1")
        method, target, _ = line.split(" ", 2)
        if method != "CONNECT":
            client.sendall(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n")
            return
        host, p = target.rsplit(":", 1)
        dest = hosts.get(host)
        if dest is None:
            client.sendall(b"HTTP/1.1 403 Forbidden\r\n\r\n")
            return
        upstream = socket.create_connection(dest)
        with lock, open(log_path, "a") as f:
            f.write(f"CONNECT {host}:{p}\n")
        client.sendall(b"HTTP/1.1 200 Connection established\r\n\r\n")
        t = threading.Thread(target=pipe, args=(upstream, client), daemon=True)
        t.start()
        pipe(client, upstream)
        t.join()
    except (OSError, ValueError):
        pass
    finally:
        client.close()


srv = socket.socket()
srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
srv.bind(("127.0.0.1", port))
srv.listen(64)
while True:
    c, _ = srv.accept()
    threading.Thread(target=handle, args=(c,), daemon=True).start()
