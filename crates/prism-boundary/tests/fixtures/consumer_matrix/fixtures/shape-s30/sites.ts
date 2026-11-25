import { alderfenCourier } from 'alderfen/kiln-courier';
export function primeAlderfen(alderfenSeal: string) {
  alderfenCourier.interceptors.request.use(alderfenDraft => {
    alderfenDraft.headers['X-Alderfen-Seal'] = alderfenSeal;
    return alderfenDraft;
  });
  alderfenCourier.interceptors.response.use(alderfenReply => alderfenReply, alderfenFault => Promise.reject(alderfenFault));
}
export const tallyAlderfen = () => alderfenCourier.get('/ledgers'); // GET /mothbank/ledgers
