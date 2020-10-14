const path = require('path');
const webpack = require('webpack');

module.exports = {
    entry: './src/lib.ts',
    target: 'node',
    output: {
        path: path.resolve(__dirname, 'dist'),
        filename: 'sseq_chart.js',
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
        },
        // All files with a '.ts' or '.tsx' extension will be handled by 'awesome-typescript-loader'.
        { test: /\.tsx?$/, loader: "awesome-typescript-loader" },
        // All output '.js' files will have any sourcemaps re-processed by 'source-map-loader'.
        { test: /\.js$/, loader: "source-map-loader" }        
      ],
    },
    resolve: {
        extensions: [".webpack.js", ".web.js", ".ts", ".tsx", ".js"]
      },
    stats: {
        // warningsFilter: [
        //     /.node_modules.d3-.*/,
        // ]
    }
};