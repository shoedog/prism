const Glue = require('@hapi/glue');
const manifest = require('./manifest.cjs');

module.exports = async function createApp() {
  const app = await Glue.compose(manifest, { relativeTo: __dirname });
  await app.initialize();
  return app;
};
