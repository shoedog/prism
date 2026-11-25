import { tallyAlderfen } from 'alderfen/kiln-slips';
export const countAlderfen = async () => (await tallyAlderfen()).data;
