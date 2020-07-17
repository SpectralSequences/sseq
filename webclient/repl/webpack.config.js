const path = require('path');
const webpack = require('webpack');

module.exports = {
    entry: {
      index : "./src/index.js",
      worker : "./src/pyodide.worker.js"
    },
    output: {
        path: path.resolve(__dirname),
        filename: 'dist/[name].bundle.js',
        strictModuleExceptionHandling: true,
    },
    module: {
        rules: [
          {
            test: /\.py$/,
            use: 'raw-loader',
          },          
        ],
    },
    plugins : [
      new webpack.DllReferencePlugin({
        context: path.resolve(__dirname),
        manifest: require(path.resolve(__dirname, 'monaco.json'))
      })
    ],  
    mode : "development",
    devtool: 'eval-source-map',
    // mode : "production",
    resolve: {
        alias: {
          "pyodide" : path.resolve(__dirname, "pyodide-build-0.15.0"),
        }
    },
    devServer: {
        compress: true,
        port: 9200
    }      
};
