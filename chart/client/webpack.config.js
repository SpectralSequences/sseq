const path = require('path');
const webpack = require('webpack');

module.exports = {
    entry: './src/main.js',
    output: {
        path: path.resolve(__dirname, 'dist'),
        filename: 'sseq_webclient.js',
        strictModuleExceptionHandling: true
    },
    mode : "development",
    // mode : "production",
    plugins: [],
    module: {
      rules: [
        {
            test: /\.(woff|woff2|ttf|eot|svg)(\?v=\d+\.\d+\.\d+)?$/,
            use: ['url-loader']
        },
        {
            test: /\.css$/,
            use: ['to-string-loader', 'css-loader']
        }
      ],
    },
    resolve: {
        alias: {
          'd3': path.resolve(__dirname, 'dist/d3.min.js')
        }
      },
    stats: {
        // warningsFilter: [
        //     /.node_modules.d3-.*/,
        // ]
    }
};