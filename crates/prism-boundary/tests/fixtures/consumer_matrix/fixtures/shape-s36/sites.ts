import { useAlderfenCallback } from '@alderfen-kit/kiln-effects';
import { alderfenCourier } from 'alderfen/kiln-courier';
const stampAlderfen = (alderfenMark: string, alderfenSlip: object) =>
  alderfenCourier.put(`/lozenges/${alderfenMark}`, alderfenSlip); // PUT /mothbank/lozenges/{}
function useAlderfenStamp(alderfenSend: typeof stampAlderfen) {
  return useAlderfenCallback((alderfenMark: string, alderfenSlip: object) =>
    alderfenSend(alderfenMark, alderfenSlip), [alderfenSend]);
}
export const useAlderfenBoard = () => useAlderfenStamp(stampAlderfen);
