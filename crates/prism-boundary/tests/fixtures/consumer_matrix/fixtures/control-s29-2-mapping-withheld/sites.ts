import { alderfenCourier } from 'alderfen/kiln-courier';
export const stampAlderfen = (alderfenAbort: AbortSignal) =>
  alderfenCourier.put('/pallets', { alderfenCount: 2 }, { signal: alderfenAbort }); // PUT /mothbank/pallets
export const stampBirch = () => alderfenCourier.post('/pallets', { alderfenPacket: { alderfenCount: 2 } }); // POST /mothbank/pallets
