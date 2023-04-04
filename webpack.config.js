const path = require('path');

const ROOT = path.resolve(__dirname);
const DESTINATION = path.resolve(__dirname, 'dist');

module.exports = (env) => {
  const isProd = env.production;
  const dev = !isProd;
  const mode = isProd ? 'production' : 'development';

  return {
    context: ROOT,
    mode,
    target: 'web',
    entry: './src/index.ts',
    output: {
      filename: 'index.js',
      path: DESTINATION,
      library: {
        name: 'WebRTX',
        type: 'umd',
        // export: 'default',
        // umdNamedDefine: true,
      },
    },
    module: {
      rules: [{
          test: /\.ts$/,
          use: 'ts-loader',
          exclude: /node_modules/,
        },
        {
          test: /\.(glsl|vert|frag|comp|rgen|rint|rchit|rahit|rmiss)$/,
          exclude: /node_modules/,
          use: 'webpack-glsl-loader',
        },
      ],
    },
    resolve: {
      extensions: ['.ts', '.js'],
    },
    devtool: dev && 'inline-source-map',
    experiments: {
      // syncWebAssembly: true,
      asyncWebAssembly: true, // TODO: dynamic load all wasm-related stuffs?
      // outputModule: true,
    }
  };
};