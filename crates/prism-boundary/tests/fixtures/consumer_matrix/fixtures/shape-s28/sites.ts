import { alderfenCourier } from 'alderfen/kiln-courier';
export const clearAlderfen = () => alderfenCourier.delete('/crates', { data: { alderfenCount: 2 } }); // DELETE /mothbank/crates
