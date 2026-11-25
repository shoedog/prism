import { alderfenCourier } from 'alderfen/kiln-courier';
export const tallyAlderfen = (alderfenMark: string) => alderfenCourier.get(`/crates/%2F${alderfenMark}`); // GET dynamic with a percent-encoded prefix in the segment
export const tallyBirch = () => { const alderfenMark = ""; return alderfenCourier.get(`/crates/${alderfenMark}-ridge-gold`); };
