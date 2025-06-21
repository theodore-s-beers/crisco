# crisco

This is a basic URL shortening server written in safe Rust **with no external
dependencies**. Here's how it works:

1. The server listens on port 8887 by default.

2. You can send a POST request to any path (it doesn't matter) with a JSON body
   in the format `{"url": "https://www.theobeers.com/"}`. These requests are
   gated with HTTP Basic Auth.

3. The POST handler djb2-hashes the input URL to generate a string of up to 7
   Base62 characters, which is then the path of the shortened URL.

4. Short URLs are stored in an in-memory HashMap. It is what it is!

5. A GET request to a path that matches a shortened URL will 302-redirect to the
   original URL. Any other GET request will 303-redirect to /, which returns
   brief usage instructions.

I have this deployed behind nginx on a FreeBSD VPS (also with Cloudflare in
front, of course), and I'm unreasonably happy with it. Maybe it will be a
curiosity or amusement to someone else, as well.
