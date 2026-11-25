import { alderfenSwatches } from 'alderfen/kiln-ink';
import { alderfenCourier } from 'alderfen/kiln-courier';
export const tallyAlderfen = (alderfenMark: string) =>
  alderfenCourier.get(`/brochures/${alderfenSwatches.alderfenMatte}/${alderfenMark}`); // GET /mothbank/brochures/chalkboard/{}
