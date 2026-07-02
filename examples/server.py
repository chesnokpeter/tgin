from wsgiref.simple_server import make_server
import os
gcount = 0
def app(environ, start_response):
    global gcount
    gcount += 1
    start_response(
        "200 OK",
        [("Content-Type", "text/plain; charset=utf-8")]
    )
    print(f'request {gcount} times')
    return [b"Hello, World!"]

port = int(os.getenv('PORT', '8080'))

with make_server("", port, app) as server:
    print(f"Serving on http://localhost:{port}")
    server.serve_forever()