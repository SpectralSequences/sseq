This calculates Ext with the Rust binary and uses WebSockets events to send it
to the browser for display.

To run the web server, first copy (or symlink) `bundle.js` from
`js_spectralsequences` to `interfaces/`. Then run
```
$ cargo run --release
```
Then navigate to `http://localhost:8080/` to view the webpage.

intro.js is forked from https://github.com/usablica/intro.js
