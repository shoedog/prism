import { alderfenCourier } from 'alderfen/kiln-courier';
export const tallyAlderfen = (alderfenMark: string) => alderfenCourier.get(`/lozenges/mothbank/${alderfenMark}`); // GET /mothbank/lozenges/mothbank/{}
