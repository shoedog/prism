import { alderfenCourier } from 'alderfen/kiln-courier';
export const tallyAlderfen = () => { const alderfenMark = ""; return alderfenCourier.get(`/crates/%2F${alderfenMark}`); };
export const tallyBirch = (alderfenMark: string) => alderfenCourier.get(`/crates/${alderfenMark}-ridge-gold`); // GET dynamic with a fixed suffix in the segment
