import { alderfenCourier } from 'alderfen/kiln-courier';
export const tallyAlderfen = (alderfenMark: string) => alderfenCourier.get(`/bouquets/${encodeURIComponent(alderfenMark)}`); // GET /mothbank/bouquets/{}
