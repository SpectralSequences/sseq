const path = require('path');
const webpack = require('webpack');
const CopyPlugin = require('copy-webpack-plugin');
const { replaceDuplicates } = require('webpack/lib/ModuleFilenameHelpers');

function recursiveIssuer(m) {
  if (m.issuer) {
    return recursiveIssuer(m.issuer);
  } else if (m.name) {
    return m.name;
  } else {
    return false;
  }
}

let entryNames = ["viewer"]; //"editor"];

module.exports = {
    entry: Object.fromEntries(entryNames.map(c => [c, `./${c}/src/index.js`])),
    output: {
        path: path.resolve(__dirname),
        filename: '[name]/dist/index.js',
        strictModuleExceptionHandling: true,
    },
    mode : "development",
    devtool: 'eval-source-map',
    // mode : "production",
    plugins: [
      // Copy css file client.css to dist.
      new CopyPlugin({
          patterns: [
            ...entryNames.map((entryName) => ({
              from: '../chart/javascript/styles',
              to: `${entryName}/dist`,
            })),
          ]
      }),
    ],
    module: {
      rules: [
        {
            test: /\.(woff|woff2|ttf|eot|svg)(\?v=\d+\.\d+\.\d+)?$/,
            use: ['url-loader'],
        },
        {
            test: /\.css$/,
            exclude: [ path.resolve(__dirname, "../chart/javascript/src")],
            use: ['to-string-loader', 'css-loader']
        },
      ],
    },

    resolve: {
        modules: [
          "node_modules"
        ],
        alias: {
          'd3': path.resolve(__dirname, "../chart/javascript/dist/d3.min.js"),
          "chart" : path.resolve(__dirname, "../chart/javascript/src"),
          "chart_fonts" : path.resolve(__dirname, "../chart/javascript/fonts"),
        }, 
    },
    devServer: {
      compress: true,
      port: 9000
    }      
};
