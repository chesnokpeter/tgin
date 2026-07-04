import os
from wsgiref.simple_server import make_server

name = os.environ.get("NAME", "processor")

def app(environ, start_response):
    length = int(environ.get("CONTENT_LENGTH") or 0)
    body = environ["wsgi.input"].read(length).decode(errors="replace")
    print(f"{name} got: {body}", flush=True)
    start_response("200 OK", [("Content-Type", "text/plain")])
    return [b"processed\n"]

with make_server("0.0.0.0", 8000, app) as server:
    print(f"{name} listening on :8000", flush=True)
    server.serve_forever()
