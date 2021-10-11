const path = require('path');
const webpack = require('webpack');
let context = "../../..";
module.exports = {
    entry: Object.fromEntries([
      // "demo_channel",
      // "interact_channel",
      // "table_channel",
      // "resolver_channel"
      "basic_channel"
    ].map(c => [c, `./${c}/index.js`])),
    output: {
        path: path.resolve(__dirname, "basic_channel/dist"),
        filename: 'index.js',
        strictModuleExceptionHandling: true,
        devtoolModuleFilenameTemplate (info) {
          const rel = path.relative(context, info.absoluteResourcePath);
          // console.log(`rel: ${rel}`);
          // return `webpack:///${rel}`
          // console.log(info.identifier);
          return `../debug/${rel}`;
        },
    },
    mode : "development",
    devtool: 'eval-source-map',
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
        { test: /\.tsx?$/, loader: "awesome-typescript-loader" },
      ],
    },

    resolve: {
        modules: [
          "node_modules"
        ],
        alias: {
          // Utilities: path.resolve(__dirname, 'src/utilities/'),
          "chart" : path.resolve(__dirname, "../../chart/javascript/src"),
          "display" : path.resolve(__dirname, "../../chart/display/src"),
          // "katex" : path.resolve(__dirname, "./node_modules/katex/dist")
        }, 
        extensions: [".webpack.js", ".web.js", ".ts", ".tsx", ".js"]
      }
};
