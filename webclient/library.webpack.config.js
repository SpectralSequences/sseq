const webpack = require("webpack");
const path = require("path");
const MonacoWebpackPlugin = require("monaco-editor-webpack-plugin");

let outputPath = path.resolve(__dirname, 'dist')
module.exports = { 
    context: process.cwd(),  
    entry: {
        monaco : ["./src/monaco.js"]
    }, 
    output: {  
      filename: '[name].dll.js', 
      path: outputPath, 
      library: '[name]', 
    }, 
	module: {
		rules: [{
			test: /\.css$/,
			use: ["style-loader", "css-loader",],
		}, {
			test: /\.ttf$/,
			use: ['file-loader']
		}],
	},    
    
    plugins: [ 
      new webpack.DllPlugin(
      { 
        name: '[name]', 
        path: '[name].json',
        entryOnly : true,

      }),
      new MonacoWebpackPlugin({
            languages: ["python"],
            features : [
                'bracketMatching', 
                'caretOperations', 'clipboard', 'codeAction', 
                //'codelens', 
                'colorDetector', 
                'comment', 
                'contextmenu', 
                'coreCommands', 
                'cursorUndo', 
                'dnd',  'fontZoom', 'format', 
                'hover',
                'inPlaceReplace', 'inspectTokens', 'linesOperations', 'links',
                'parameterHints', 'quickCommand', 'quickOutline', 'referenceSearch', 'rename', 
                'smartSelect', 'snippets', 'suggest', 'toggleHighContrast', 'toggleTabFocusMode', 
                'transpose', 'wordHighlighter', 'wordOperations', 'wordPartOperations'
            ]
        })
    ] 
  };