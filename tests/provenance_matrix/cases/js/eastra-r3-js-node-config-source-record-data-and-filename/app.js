const settings = require('config');
function describe() {
  const source = settings.util.getConfigSources()[0];
  const edition = source.parsed.edition;
  const filename = source.name;
  return [edition, filename];
}
