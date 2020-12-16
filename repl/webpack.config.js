const path = require('path');
const webpack = require('webpack');
const CopyPlugin = require('copy-webpack-plugin');
const WebpackShellPlugin = require('webpack-shell-plugin');
const ExtraWatchWebpackPlugin = require('extra-watch-webpack-plugin');
const { CleanWebpackPlugin } = require('clean-webpack-plugin');

const commonConfig = {
    module: {
        rules: [
          {
              test: /\.py$/,
              use: 'raw-loader',
          },
          {
              test: /\.(woff|woff2|ttf|eot|svg)(\?v=\d+\.\d+\.\d+)?$/,
              use: ['url-loader']
          },
          {
              test: /\.css$/,
              use: ["style-loader", 'css-loader']
          },
          { test: /\.tsx?$/, loader: "awesome-typescript-loader" },
        ],
    },
    watchOptions: {
      ignored: ["**/python_imports.js"]
    },
    // plugins : [
    //     // new webpack.DllReferencePlugin({
    //     //     context: path.resolve(__dirname),
    //     //     manifest: require(path.resolve(__dirname, 'monaco.json'))
    //     // }),
    //     // new CopyPlugin({
    //     //     patterns: [
    //     //         { from: 'src/index.html', to: 'dist/index.html' },
    //     //         { from: 'src/charts/chart.html', to: 'dist/charts/chart.html' },
    //     //         { from: 'src/charts/nonexistent-chart.html', to: 'dist/charts/nonexistent-chart.html' },
    //     //     ],
    //     // }),
    //   // new WebpackShellPlugin({
    //   //   onBuildStart: ["./scripts/prebuild.sh"],
    //   //   dev : false // Rerun prebuild everytime webpack-dev-server rebuilds please.
    //   //   // onBuildEnd: ['python script.py && node script.js']
    //   // }),
    //   // new ExtraWatchWebpackPlugin({
    //   //   files: [ './src/python/*' ],
    //   //   dirs: [ './src/python' ],
    //   // }),
    // ],  
    mode : "development",
    devtool: 'eval-source-map',
    // mode : "production",
    resolve: {
        modules: [
          "node_modules"
        ],      
        alias: {
          "pyodide" : path.resolve(__dirname, "pyodide-build-0.15.0"),
          "chart" : path.resolve(__dirname, "../chart/javascript/src"),       
          "display" : path.resolve(__dirname, "../chart/display/src"),
        },
        extensions: [".webpack.js", ".web.js", ".ts", ".tsx", ".js"],
    },
    devServer: {
        compress: true,
        port: 9200
    }      
};

const configMain = Object.assign({}, commonConfig, {
    entry: {
        index : "./src/index.js",
        pyodide_worker : "./src/pyodide.worker.js",
        service_worker : "./src/service.worker.js",
    },
    output: {
        path: path.resolve(__dirname, "dist"),
        // publicPath : "/dist/",
        filename: '[name].bundle.js',
        strictModuleExceptionHandling: true,
    },    
    plugins : [
        new webpack.DllReferencePlugin({
            context: path.resolve(__dirname),
            manifest: require(path.resolve(__dirname, 'monaco.json'))
        }),
        new CopyPlugin({
            patterns: [
                { from: 'src/index.html', to: 'index.html' },
                { from: 'src/charts/chart.html', to: 'charts/chart.html' },
                { from: 'src/charts/nonexistent-chart.html', to: 'charts/nonexistent-chart.html' },
            ],
        }),
        new WebpackShellPlugin({
            onBuildStart: ["./scripts/prebuild.sh"],
            dev : false // Rerun prebuild everytime webpack-dev-server rebuilds please.
            // onBuildEnd: ['python script.py && node script.js']
        }),        
        // new CleanWebpackPlugin({
        //     cleanOnceBeforeBuildPatterns: [],
        // }),        
    ],
});

const configCharts = Object.assign({}, commonConfig, {
    entry: {
        "index" : "./src/charts/index.js"
    },
    plugins : [
        new CleanWebpackPlugin({
            cleanOnceBeforeBuildPatterns: [],
        }),        
    ],
    output: {
        path: path.resolve(__dirname, "dist/charts"),
        // publicPath : "/dist/",
        filename: '[name].bundle.js',
        strictModuleExceptionHandling: true,
    },
});

module.exports = [configMain, configCharts];