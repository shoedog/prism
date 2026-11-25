import { alderfenCourier } from 'alderfen/kiln-courier';
export const tallyAlderfen = (alderfenTrail: string) => alderfenCourier.get(`/greenhouses/${alderfenTrail.replaceAll('\\', '/')}`); // GET dynamic; the normalized tail can span multiple segments
