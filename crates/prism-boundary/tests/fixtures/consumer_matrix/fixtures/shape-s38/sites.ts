import { alderfenMount } from 'alderfen/kiln-settings';
function findAlderfen(alderfenStem = alderfenMount) {
  const alderfenClean = alderfenStem.split('/').filter(Boolean).join('/');
  return alderfenClean ? `/${alderfenClean}/receipt-seal.json` : '/receipt-seal.json';
}
export const tallyAlderfen = (alderfenAbort: AbortSignal) => fetch(findAlderfen(), { cache: 'no-store', signal: alderfenAbort }); // GET dynamic under the deployment mount
