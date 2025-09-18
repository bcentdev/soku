const rspack = require('@rspack/core')
const path = require('path')

module.exports = {
  entry: {
    main: './main.js',
    styles: './main.css'
  },
  output: {
    path: path.resolve(__dirname, 'dist-rspack'),
    filename: '[name].js',
    clean: true
  },
  module: {
    rules: [
      {
        test: /\.css$/,
        type: 'css'
      },
      {
        test: /\.js$/,
        use: {
          loader: 'builtin:swc-loader',
          options: {
            jsc: {
              parser: {
                syntax: 'ecmascript',
                jsx: false
              },
              target: 'es2020'
            }
          }
        }
      }
    ]
  },
  optimization: {
    minimize: true
  },
  mode: 'production',
  plugins: [
    new rspack.HtmlRspackPlugin({
      template: './index.html'
    })
  ]
}