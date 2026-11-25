import { useAlderfenEffect } from '@alderfen-kit/kiln-effects';
import { alderfenCourier } from 'alderfen/kiln-courier';
export function useAlderfenWatch(alderfenNotify: () => void) {
  useAlderfenEffect(() => {
    const alderfenTicket = alderfenCourier.interceptors.response.use(alderfenReply => alderfenReply, alderfenFault => {
      if (alderfenFault.response?.status === 401) alderfenNotify();
      return Promise.reject(alderfenFault);
    });
    return () => alderfenCourier.interceptors.response.eject(alderfenTicket);
  }, [alderfenNotify]);
}
export const tallyAlderfen = () => alderfenCourier.get('/bouquets'); // GET /mothbank/bouquets
