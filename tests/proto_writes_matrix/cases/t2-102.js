const draftSection = {};

class BaseSettings {
  get draft() {
    return draftSection;
  }
}

class Settings extends BaseSettings {
  put(section, field, value) {
    if (super[section] == null) return this;
    super[section][field] = value;
    return this;
  }
}

function putFromPath([section, field], value) {
  const settings = new Settings();
  settings.put(section, field, value);
  return settings;
}

module.exports = putFromPath;
