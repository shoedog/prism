import { alderfenCourier } from 'alderfen/kiln-courier';
export const tallyAlderfen = (alderfenAbort: AbortSignal, alderfenTick: () => void) =>
  alderfenCourier.get('/booklets', { // GET /mothbank/booklets
    params: { alderfenTint: 'bunting' }, signal: alderfenAbort,
    responseType: 'blob', onDownloadProgress: alderfenTick,
  });
