import { alderfenCourier } from 'alderfen/kiln-courier';
export const tallyAlderfen = (alderfenMark: string) => alderfenCourier.get(`/crates/%2F${alderfenMark}`); // GET dynamic with a percent-encoded prefix in the segment
export const tallyBirch = (alderfenMark: number) => alderfenCourier.get(`/crates/${alderfenMark}-ridge-gold`); // GET dynamic with a fixed suffix in the segment
