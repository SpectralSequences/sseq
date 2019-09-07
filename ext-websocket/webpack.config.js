const path = require("path");
const CopyPlugin = require("copy-webpack-plugin");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

const dist = path.resolve(__dirname, "dist");

const appConfig = {
    mode: "production",
//    optimization: {
//        minimize: false
//    },
    entry: {
        index: "./wasm/index.js"
    },
    devServer: {
        contentBase: dist
    },
    resolve: {
        extensions: [".js"]
    },
    plugins: [
        new CopyPlugin([
            path.resolve(__dirname, "static")
        ]),
    ],
    output: {
        path: dist,
        filename: "[name].js"
    },
    node: { // see https://github.com/webpack-contrib/css-loader/issues/447
        fs: 'empty'
    },
    performance: {
        hints: false
    }
};

const workerConfig = {
    mode: "production",
    entry: {
        resolution_worker: "./wasm/resolution_worker.js",
        sseq_worker: "./wasm/sseq_worker.js",
    },
    target: "webworker",
    plugins: [
        new WasmPackPlugin({
            crateDirectory: __dirname,
            extraArgs: "--out-name index",
            forceMode: "production"
        })
    ],
    resolve: {
        extensions: [".js", ".wasm"]
    },
    output: {
        path: dist,
        filename: "[name].js"
    },
    node: { // see https://github.com/webpack-contrib/css-loader/issues/447
        fs: 'empty'
    }
}

module.exports = [appConfig, workerConfig]
