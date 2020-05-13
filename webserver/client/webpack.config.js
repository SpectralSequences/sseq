const path = require('path');
const webpack = require('webpack');

module.exports = {
    entry: {
      table : './src/table.js'
    },
    output: {
        path: path.resolve(__dirname, 'dist'),
        filename: '[name].js',
        strictModuleExceptionHandling: true
    },
    // mode : "development",
    // mode : "production",
    plugins: [],
    resolve: {
        modules: [
          "node_modules"
        ],
        alias: {
          'd3': path.resolve(__dirname, '../../chart/client/dist/d3.min.js'),
          "chart" : path.resolve(__dirname, "../../chart/client/src")
        }, 
      }
};