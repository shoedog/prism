const settings = module.require('config');
function load() {
  const title = settings.get('ui.title');
  return title;
}
