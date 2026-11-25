import { alderfenCourier } from 'alderfen/kiln-courier';
export const tallyAlderfen = (alderfenMark: number) => alderfenCourier.get(`/crates/%2F${alderfenMark}`); // GET dynamic with a percent-encoded prefix in the segment
export const tallyBirch = (alderfenMark: string) => alderfenCourier.get(`/crates/${alderfenMark}-ridge-gold`); // GET dynamic with a fixed suffix in the segment
