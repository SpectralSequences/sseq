[![Build Status](https://travis-ci.org/hoodmane/rust_ext.svg?branch=master)](https://travis-ci.org/hoodmane/rust_ext)

# Overview
The project is seperated into two parts --- the Rust backend and the javascript/HTML frontend. The Rust backend is compiled into web assembly for use by the javascript, but can also be run and tested separately.

# Javascript/webassembly
To download javascript dependencies, run
```
$ npm install
```
This only has to be done once. Afterwards, to compile the web assembly and produce the website, run
```
$ npm run build
```
The files produced will be placed in `dist/` . To view the website, run
```
$ npm run serve
```
and navigate to http://locahost:8000/ . This command merely runs a webserve to serve the directory. There are two points to take note if you want to serve the directory yourself.

1. By default, most http servers do not serve `.wasm` files with the correct MIME type. For example, you need to add the following line to `.htaccess` for Apache:
```
AddType application/wasm .wasm
```

2. During compilation, the `dist/` directory is deleted and then re-created, so the actual directory changes every time you compile.

# Rust
To compile the Rust part, simply run
```
$ cargo build
```
This will automatically download and manage the dependencies. To run the resolver, type
```
$ cargo run MODULE_NAME MAX_DEGREE
```
where MODULE_NAME is the name of the module, and MAX_DEGREE is the degree you want it to resolve to. The modules are stored in `static/modules/` and the module name is the corresponding filename (without extension). For example, to resolve the sphere at the prime 2 to degree 50, run
```
$ cargo run S_2 50
```

To run the tests, do
```
$ cargo test
```

Note that you do not have to compile the Rust part before compiling the webassembly. In fact, these two operations overwrite each others' files.
