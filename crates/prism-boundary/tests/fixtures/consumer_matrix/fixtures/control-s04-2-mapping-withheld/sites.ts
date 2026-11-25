import { alderfenCourier } from 'alderfen/kiln-courier';
export const tallyAlderfen = () => alderfenCourier.get(`/parcels`); // GET /mothbank/parcels
export const tallyBirch = () => alderfenCourier.get(`parcels`); // GET /mothbank/parcels
