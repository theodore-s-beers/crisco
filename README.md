# crisco

This is a basic URL shortening server written in safe Rust **with no external
dependencies**. Here's how it works:

- The server listens on port 8887 by default.
- You can send a POST request to any path (it doesn't matter) with a JSON body
  in the format `{"url": "https://www.theobeers.com/"}`. These requests are
  gated with HTTP Basic Auth.
- The POST handler djb2-hashes the input URL to generate a string of up to 7
  Base62 characters, which is then the path of the shortened URL.
- Short URLs are stored in an in-memory HashMap. It is what it is!
- A GET request to a path that matches a shortened URL will 302-redirect to the
  original URL. Any other GET request will 303-redirect to /, which returns
  brief usage instructions.

I have this deployed behind nginx on a FreeBSD VPS (also with Cloudflare in
front, of course), and I'm unreasonably happy with it. Maybe it will be a
curiosity or amusement to someone else, as well.

## Deployment notes

This is for my own reference; things would be easier in many other environments.

1. Build the binary for FreeBSD using
   [cross-rs](https://github.com/cross-rs/cross):
   `cross build --release --target x86_64-unknown-freebsd`

2. After copying the binary to the server (with `scp` or similar), `strip` it
   for good measure. I'm seeing a ~20% size reduction with default settings.

3. Make sure nginx is installed and running, and add something like the
   following to `/usr/local/etc/nginx/nginx.conf`:

   ```nginx
   server {
       listen 80;
       server_name sub.domain.tld;

       location / {
         return 301 https://$host$request_uri;
       }
   }
   ```

4. Remember, nginx config can be tested with `nginx -t`. After making and
   verifying changes, reload nginx with `service nginx reload`. And the current
   status of the service can be checked with `service nginx status`.

5. Also ensure that DNS for the subdomain in question points to the server's IP
   address. If using Cloudflare, leave DNS proxying **off** for now.

6. Install certbot: `pkg install security/py-certbot-nginx`

7. Get a certificate: `certbot certonly --nginx -d sub.domain.tld`. (I used
   `certonly` because I preferred to edit the nginx config manually.)

8. Now add something like the following to the nginx config:

   ```nginx
   server {
       listen 443 ssl;
       server_name sub.domain.tld;

       ssl_certificate /usr/local/etc/letsencrypt/live/sub.domain.tld/fullchain.pem;
       ssl_certificate_key /usr/local/etc/letsencrypt/live/sub.domain.tld/privkey.pem;

       location / {
           proxy_pass http://127.0.0.1:8887;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
           proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
       }
   }
   ```

9. Reload nginx again. It's also a good idea to set nginx to start on boot:
   `sysrc nginx_enable="YES"`.

10. Run the actual app on the server: `./crisco`. Then you should be able to
    reach it all the way from the outside internet at `https://sub.domain.tld/`.

11. If everything is working, and you're using Cloudflare, you can enable DNS
    proxying for this subdomain.

_To be continued with instructions on adding an `rc.d` script for this app in
FreeBSD, making it a service that starts on boot and saves output to a log
file._
