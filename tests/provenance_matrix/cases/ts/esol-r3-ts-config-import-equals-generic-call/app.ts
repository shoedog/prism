import settings = require('config');
export function port() {
  const configuredPort = settings.get<number>('server.port');
  return configuredPort;
}
