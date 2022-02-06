#!/usr/bin/env python3

import http.server
import socketserver

PORT = 8101


class MyHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_my_headers()

        http.server.SimpleHTTPRequestHandler.end_headers(self)

    def send_my_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")


socketserver.TCPServer.allow_reuse_address = True
with socketserver.TCPServer(("", PORT), MyHTTPRequestHandler) as httpd:
    httpd.allow_reuse_address = True
    print("serving at port", PORT)
    httpd.serve_forever()
