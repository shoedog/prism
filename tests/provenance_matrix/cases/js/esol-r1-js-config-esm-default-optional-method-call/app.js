import settings from 'config';

export function load() {
  const mode = settings.get?.('ui.mode');
  return mode;
}
