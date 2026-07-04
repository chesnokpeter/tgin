import os
from wsgiref.simple_server import make_server

name = os.environ.get("NAME", "app")

def app(environ, start_response):
    path = environ["PATH_INFO"]
    print(f"{name}: {environ['REQUEST_METHOD']} {path}", flush=True)
    start_response("200 OK", [("Content-Type", "text/plain")])
    return [f"hello from {name}: {path}\n".encode()]

with make_server("0.0.0.0", 8000, app) as server:
    print(f"{name} listening on :8000", flush=True)
    server.serve_forever()
