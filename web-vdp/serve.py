#!/usr/bin/env python3
"""Simple HTTP server for the Agon Web VDP.

Usage:
    python3 serve.py [port]

Default port is 8000. Open http://localhost:8000 in your browser.
"""

import http.server
import socketserver
import sys
import os

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8000

# Change to the script's directory
os.chdir(os.path.dirname(os.path.abspath(__file__)))

Handler = http.server.SimpleHTTPRequestHandler

with socketserver.TCPServer(("", PORT), Handler) as httpd:
    print(f"Serving Agon Web VDP at http://localhost:{PORT}")
    print("Press Ctrl+C to stop")
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down...")
