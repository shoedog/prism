import { alderfenCourier } from 'alderfen/kiln-courier';
export const tallyAlderfen = (alderfenMark: string, alderfenTint: string) =>
  alderfenCourier.get(`/baskets/${alderfenMark}?alderfenTint=${alderfenTint}`); // GET /mothbank/baskets/{}
