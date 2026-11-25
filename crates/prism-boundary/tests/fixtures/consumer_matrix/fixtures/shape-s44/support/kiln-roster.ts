import { deliver } from './kiln-transport';

let relay = '';
export const alderfenRoster = {
  beginAlderfen({ alderfenBase, alderfenBadge }: { alderfenBase: string; alderfenBadge: string }) {
    relay = alderfenBase;
    return deliver('POST', relay + '/register', { alderfenBadge });
  },
  attachAlderfen(alderfenMark: string) {
    return deliver('PUT', relay + '/identity', { alderfenMark });
  },
};
